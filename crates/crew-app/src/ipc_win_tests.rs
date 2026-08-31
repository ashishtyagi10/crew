use super::*;

/// The endpoint keeps the socket file name — that is what `ipc.rs` parses
/// the instance id back out of — and re-roots it in the pipe namespace, so
/// a caller may hand us any directory and still name a valid pipe.
#[test]
fn pipe_names_are_rooted_in_the_pipe_namespace() {
    let name = |p: &str| pipe_name(Path::new(p)).to_string_lossy().into_owned();
    assert_eq!(name(r"\\.\pipe\crew-ipc.sock"), r"\\.\pipe\crew-ipc.sock");
    // A temp-dir path (what the round-trip test uses) still names a pipe.
    assert_eq!(
        name(r"C:\Users\x\AppData\Local\Temp\crew-ipc-alpha.sock"),
        r"\\.\pipe\crew-ipc-alpha.sock"
    );
    // A path with no file name cannot name a pipe; fall back to the default.
    assert_eq!(name(r"C:\"), r"\\.\pipe\crew-ipc.sock");
}

/// Win32 wants UTF-16 with a terminator, and exactly one.
#[test]
fn wide_names_are_nul_terminated() {
    let w = wide_pipe_name(Path::new(r"\\.\pipe\crew-ipc.sock"));
    assert_eq!(w.last(), Some(&0));
    assert_eq!(w.iter().filter(|c| **c == 0).count(), 1);
    assert_eq!(
        String::from_utf16_lossy(&w[..w.len() - 1]),
        r"\\.\pipe\crew-ipc.sock"
    );
}
