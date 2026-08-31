//! The Windows transport for the inter-pane `ask` IPC: a named pipe standing in
//! for the Unix-domain socket, shaped to the same API so [`crate::ipc`] itself
//! stays platform-neutral.
//!
//! The endpoint is still addressed by a [`Path`] — Windows names pipes with
//! one (`\\.\pipe\crew-ipc.sock`), so the socket-name, instance-id and
//! discovery logic in `ipc.rs` is shared verbatim between both platforms.
//! Only the file name of the path is significant; a caller (the round-trip
//! test) may hand us a temp-dir path and still get a valid, unique pipe.
//!
//! Two deliberate differences from the Unix socket:
//!
//! * [`PipeStream::set_read_timeout`] is a no-op. Timeouts on a named pipe
//!   need overlapped I/O, which the blocking one-thread-per-connection shape
//!   here does not use. The cost is that a client which connects and then
//!   never writes pins its handler thread for the life of the process instead
//!   of 300s. Only the same desktop user can open the pipe, and every real
//!   client (`crew ask` / `crew panes`) writes its request immediately.
//! * The first instance is created with `FILE_FLAG_FIRST_PIPE_INSTANCE`, so a
//!   second crew binding the same name fails rather than silently stealing it
//!   — the same protection `UnixListener::bind` gets from an existing file.
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::time::Duration;

use windows_sys::Win32::Foundation::{ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    FlushFileBuffers, FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

/// Buffer hint handed to `CreateNamedPipeW` — advisory, not a message cap.
const BUF: u32 = 64 * 1024;

/// `\\.\pipe\<file name of `path`>`, NUL-terminated UTF-16 for the Win32 call.
/// Only the file name is used: it is the part `ipc.rs` derives the instance id
/// from, and it keeps a temp-dir path from naming an unopenable pipe.
fn pipe_name(path: &Path) -> std::ffi::OsString {
    let leaf = path
        .file_name()
        .unwrap_or_else(|| OsStr::new("crew-ipc.sock"));
    let mut name = std::ffi::OsString::from(r"\\.\pipe\");
    name.push(leaf);
    name
}

/// [`pipe_name`] as the NUL-terminated UTF-16 the Win32 calls want.
fn wide_pipe_name(path: &Path) -> Vec<u16> {
    pipe_name(path).encode_wide().chain(Some(0)).collect()
}

/// One connected end of the pipe. A `File` over the pipe handle: named pipes
/// are read and written with the ordinary file API on both ends.
pub(crate) struct PipeStream(File);

impl PipeStream {
    /// Open the client end. `std::fs::OpenOptions` speaks to a named pipe
    /// directly, so the client side needs no Win32 at all.
    pub(crate) fn connect(path: &Path) -> io::Result<Self> {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(pipe_name(path))
            .map(PipeStream)
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(PipeStream)
    }

    /// No-op — see the module docs for why a named pipe has no cheap timeout
    /// here. `Ok` so the shared call site in `ipc.rs` needs no `cfg`.
    pub(crate) fn set_read_timeout(&self, _dur: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
}

impl Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Read for &PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.0).read(buf)
    }
}

/// Block until the peer has consumed everything written to `pipe`.
///
/// This is load-bearing, not hygiene: closing a named-pipe handle disconnects
/// it, and a disconnect **discards** bytes the peer has not read yet. Without
/// this, `handle_conn` writing the verdict and returning would race the client
/// into losing the reply. `File::flush` is a no-op and would not save us.
/// A peer that dies instead of reading ends the wait rather than hanging it.
fn flush_to_peer(file: &File) -> io::Result<()> {
    // SAFETY: a live pipe handle owned by `file` for the length of the call.
    let ok = unsafe { FlushFileBuffers(file.as_raw_handle() as HANDLE) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

impl Write for PipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        flush_to_peer(&self.0)
    }
}

impl Write for &PipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.0).write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        flush_to_peer(&self.0)
    }
}

/// Create one unconnected instance of the pipe. `first` asks the kernel to
/// fail if the name is already served, which is how we detect a second crew.
fn create_instance(name: &[u16], first: bool) -> io::Result<OwnedHandle> {
    let mode = PIPE_ACCESS_DUPLEX
        | if first {
            FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            0
        };
    // SAFETY: `name` is NUL-terminated UTF-16 from `wide_pipe_name`, and a
    // null `lpsecurityattributes` asks for the default descriptor (this user).
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            BUF,
            BUF,
            0,
            std::ptr::null(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a valid, exclusively-owned handle from `CreateNamedPipeW`.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle as _) })
}

/// The listening end. Holds one already-created instance so the pipe exists —
/// and is therefore discoverable in `\\.\pipe\` — before any client connects.
pub(crate) struct PipeListener {
    name: Vec<u16>,
    /// The instance the next `accept` will wait on.
    pending: Option<OwnedHandle>,
}

impl PipeListener {
    /// Bind the pipe named by `path`, failing if another process already
    /// serves that name.
    pub(crate) fn bind(path: &Path) -> io::Result<Self> {
        let name = wide_pipe_name(path);
        let pending = Some(create_instance(&name, true)?);
        Ok(PipeListener { name, pending })
    }

    /// Wait for a client on the pending instance, then create the next one so
    /// the pipe never disappears between connections.
    fn accept(&mut self) -> io::Result<PipeStream> {
        let handle = match self.pending.take() {
            Some(h) => h,
            None => create_instance(&self.name, false)?,
        };
        // SAFETY: `handle` is a live pipe instance we own; a null OVERLAPPED
        // asks for the blocking connect this thread wants.
        let ok =
            unsafe { ConnectNamedPipe(handle.as_raw_handle() as HANDLE, std::ptr::null_mut()) };
        if ok == 0 {
            let err = io::Error::last_os_error();
            // The client can win the race between create and connect; that is
            // a connected pipe, not a failure.
            if err.raw_os_error() != Some(ERROR_PIPE_CONNECTED as i32) {
                return Err(err);
            }
        }
        // Pre-create the next instance so the pipe stays present in
        // `\\.\pipe\` (and so discovery keeps working) between connections.
        // Best-effort: a failure here must not throw away the live connection
        // we just accepted — the next `accept` retries and reports it.
        self.pending = create_instance(&self.name, false).ok();
        // `OwnedHandle` moves into the `File`, which then owns the close — the
        // handle is never closed twice.
        Ok(PipeStream(File::from(handle)))
    }

    /// Blocking iterator of connections, mirroring `UnixListener::incoming`.
    pub(crate) fn incoming(mut self) -> impl Iterator<Item = io::Result<PipeStream>> {
        std::iter::from_fn(move || Some(self.accept()))
    }
}

#[cfg(test)]
#[path = "ipc_win_tests.rs"]
mod tests;
