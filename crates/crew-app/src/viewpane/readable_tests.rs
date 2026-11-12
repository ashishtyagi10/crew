use super::readable;

fn file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("crew-readable-{}-{name}", std::process::id()));
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn text_of_every_kind_is_read_here() {
    for (name, body) in [
        ("notes.md", "# Notes\n\nsome *prose*\n"),
        ("main.rs", "fn main() {}\n"),
        ("data.json", "{\"a\": 1}\n"),
        ("table.csv", "a,b\n1,2\n"),
        ("README", "plain words, no extension\n"),
    ] {
        assert!(readable(&file(name, body.as_bytes())), "{name}");
    }
}

#[test]
fn a_picture_is_read_here_too() {
    let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    assert!(readable(&file("shot.png", &png)));
}

/// What the viewer could only apologise for goes to the system instead.
#[test]
fn binaries_documents_and_missing_files_are_not() {
    assert!(!readable(&file(
        "tool.bin",
        &[0x7f, b'E', b'L', b'F', 0, 0, 1]
    )));
    assert!(!readable(&file("paper.pdf", b"%PDF-1.7\n")));
    assert!(!readable(std::path::Path::new("/no/such/crew/file.md")));
}
