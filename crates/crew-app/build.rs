//! Embeds the app icon into crew.exe on Windows builds. No-op elsewhere.
fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon/crew.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon/crew.ico");
        res.compile().expect("embed assets/icon/crew.ico");
    }
}
