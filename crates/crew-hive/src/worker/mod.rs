//! Worker codec: `LoopbackTransport` (in-process) and `serve_stdio` (sidecar line protocol).
use crate::wire::{RemoteReply, RemoteTask, Transport, TransportError};
use std::future::Future;
use std::pin::Pin;

#[cfg(test)]
mod tests;

/// In-process transport: runs `handler(task)` directly — useful for tests
/// and same-process workers.
pub struct LoopbackTransport<F> {
    pub handler: F,
}

impl<F> Transport for LoopbackTransport<F>
where
    F: Fn(RemoteTask) -> RemoteReply + Send + Sync + 'static,
{
    fn dispatch(
        &self,
        task: RemoteTask,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteReply, TransportError>> + Send>> {
        let reply = (self.handler)(task);
        Box::pin(std::future::ready(Ok(reply)))
    }
}

/// Stdio worker codec. Reads `RemoteTask` JSON lines from `reader`, calls
/// `handler`, writes `RemoteReply` JSON + `\n` to `writer`, flushes after
/// each reply. Skips unparseable lines (logs to stderr). Returns on EOF.
pub fn serve_stdio<R, W>(
    reader: R,
    mut writer: W,
    handler: impl Fn(RemoteTask) -> RemoteReply,
) -> std::io::Result<()>
where
    R: std::io::BufRead,
    W: std::io::Write,
{
    for line in std::io::BufRead::lines(reader) {
        let line = line?;
        match serde_json::from_str::<RemoteTask>(&line) {
            Ok(task) => {
                let reply = handler(task);
                let json = serde_json::to_string(&reply).map_err(std::io::Error::other)?;
                writeln!(writer, "{json}")?;
                writer.flush()?;
            }
            Err(e) => {
                eprintln!("serve_stdio: skipping unparseable line: {e}");
            }
        }
    }
    Ok(())
}
