//! The FFI cannot be exercised off Windows, but the *rule* it applies can —
//! and the rule is where the damage would be. Getting `needs_console_handle`
//! backwards would not fail to compile and would not crash; it would quietly
//! redirect piped output to a console window, breaking `crew --list-fonts >
//! file` and every broker spawn, which speaks JSON lines over pipes.
use super::needs_console_handle;

#[test]
fn a_process_with_no_handle_gets_the_console() {
    // `GetStdHandle` reports "nothing here" two ways, and a GUI-subsystem
    // process launched from Explorer gets the null one.
    assert!(needs_console_handle(0), "null handle means no stdio yet");
    assert!(
        needs_console_handle(-1),
        "INVALID_HANDLE_VALUE (-1) also means no stdio yet"
    );
}

#[test]
fn an_existing_handle_is_never_replaced() {
    // Any other value is a real handle the shell installed — a pipe, a file,
    // or an inherited console. Overwriting it with CONOUT$ would send output
    // to a console window instead of the pipe the caller is reading.
    for h in [1_isize, 3, 7, 12, 0x1234, isize::MAX] {
        assert!(
            !needs_console_handle(h),
            "handle {h} is usable — redirecting it would break piped output"
        );
    }
}

/// The GUI subsystem is what removes the console flash; if that attribute is
/// ever dropped, this module's whole reason to exist goes with it and Windows
/// starts flashing a black window on every launch again. The attribute cannot
/// be read at runtime, so assert on the source.
#[test]
fn main_still_declares_the_gui_subsystem() {
    let main_rs = include_str!("main.rs");
    assert!(
        main_rs.contains(r#"windows_subsystem = "windows""#),
        "crew-app's main.rs no longer declares the GUI subsystem — Windows \
         will create a console window before the first instruction runs, and \
         every Start-menu launch flashes it"
    );
}
