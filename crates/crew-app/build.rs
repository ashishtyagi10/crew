//! Embeds the app icon into crew.exe on Windows builds, and on macOS an
//! Info.plist into the binary (`macos/Info.plist`): the note macOS shows
//! when `/todo reminders` asks for Reminders, which it must find however
//! crew was started, from Crew.app or a shell.
fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon/crew.ico");
    println!("cargo:rerun-if-changed=macos/Info.plist");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        println!(
            "cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{dir}/macos/Info.plist"
        );
    }
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon/crew.ico");
        res.compile().expect("embed assets/icon/crew.ico");
    }
}
