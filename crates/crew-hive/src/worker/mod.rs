//! The two ends of the bridge: the loop that talks to a worker, and the loop a worker runs.
//!
//! Both are written against `BufRead`/`Write` rather than against a process, so the whole
//! conversation — a delta, a tool call, a result, a done — is testable over two in-memory
//! buffers. [`StdioTransport`] is then a thin thing that owns a child process and calls
//! [`converse`].
use crate::wire::{Host, HostMsg, RemoteReply, RemoteTask, Transport, TransportError, WorkerMsg};
use std::future::Future;
use std::io::{BufRead, Write};
use std::pin::Pin;

pub mod stdio;

#[cfg(test)]
mod tests;

/// Run one task to completion over a pair of streams.
///
/// Writes the task, then answers whatever comes back until the worker says it is done: deltas go
/// to the host's sink, calls are run through crew's tools and answered on the same stream. A
/// worker that closes without finishing is an error, not an empty answer — an engine that died
/// mid-graph must not look like one that had nothing to say.
pub fn converse<R: BufRead, W: Write>(
    task: RemoteTask,
    reader: &mut R,
    writer: &mut W,
    host: &Host<'_>,
) -> Result<RemoteReply, TransportError> {
    send(writer, &HostMsg::Task(task))?;
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| TransportError::Recv(e.to_string()))?;
        if n == 0 {
            return Err(TransportError::Recv(
                "the worker stopped before it finished the task".into(),
            ));
        }
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<WorkerMsg>(&line) {
            Ok(WorkerMsg::Delta { text }) => (host.on_delta)(&text),
            Ok(WorkerMsg::Call { id, tool, args }) => {
                let (output, ok) = host.call(&tool, &args);
                send(writer, &HostMsg::Result { id, output, ok })?;
            }
            Ok(WorkerMsg::Done(reply)) => return Ok(reply),
            // A line crew cannot read is the worker's problem to fix, and stopping the task over
            // it would turn a typo in a debug print into a failed graph.
            Err(e) => eprintln!("crew: ignoring an unreadable line from the worker: {e}"),
        }
    }
}

fn send<W: Write>(writer: &mut W, msg: &HostMsg) -> Result<(), TransportError> {
    let json = serde_json::to_string(msg).map_err(|e| TransportError::Send(e.to_string()))?;
    writeln!(writer, "{json}").map_err(|e| TransportError::Send(e.to_string()))?;
    writer
        .flush()
        .map_err(|e| TransportError::Send(e.to_string()))
}

/// In-process transport: runs `handler(task)` directly — useful for tests and same-process
/// workers. It cannot ask crew for a tool, which is the point: it is a stub, not an engine.
pub struct LoopbackTransport<F> {
    pub handler: F,
}

impl<F> Transport for LoopbackTransport<F>
where
    F: Fn(RemoteTask) -> RemoteReply + Send + Sync + 'static,
{
    fn dispatch<'a>(
        &'a self,
        task: RemoteTask,
        _host: Host<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteReply, TransportError>> + Send + 'a>> {
        let reply = (self.handler)(task);
        Box::pin(std::future::ready(Ok(reply)))
    }
}

/// Worker codec for a worker written in Rust: reads [`HostMsg`] lines, hands each task to
/// `handler`, and writes what it returns as a `done`. A handler that wants to stream or to ask
/// for a tool writes those lines itself — this is the one-shot shape, which is what the
/// in-process tests and the simplest workers need.
pub fn serve_stdio<R, W>(
    reader: R,
    mut writer: W,
    handler: impl Fn(RemoteTask) -> RemoteReply,
) -> std::io::Result<()>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line?;
        match serde_json::from_str::<HostMsg>(&line) {
            Ok(HostMsg::Task(task)) => {
                let reply = handler(task);
                let json = serde_json::to_string(&WorkerMsg::Done(reply))
                    .map_err(std::io::Error::other)?;
                writeln!(writer, "{json}")?;
                writer.flush()?;
            }
            // A result with nobody waiting for it: the handler above is one-shot, so this can
            // only be a worker bug. Skipped rather than fatal, same as an unreadable line.
            Ok(HostMsg::Result { .. }) => {}
            Err(e) => eprintln!("serve_stdio: skipping unparseable line: {e}"),
        }
    }
    Ok(())
}
