use super::*;

#[test]
fn a_path_with_spaces_round_trips() {
    let p = if cfg!(windows) {
        PathBuf::from(r"C:\Users\me\my project\src\main.rs")
    } else {
        PathBuf::from("/Users/me/my project/src/main.rs")
    };
    let u = from_path(&p);
    assert!(u.starts_with("file:///"), "{u}");
    assert!(u.contains("my%20project"), "{u}");
    assert_eq!(to_path(&u), Some(p));
}

#[test]
fn unicode_survives_as_percent_encoded_utf8() {
    let p = PathBuf::from("/tmp/é/x.rs");
    let u = from_path(&p);
    assert_eq!(u, "file:///tmp/%C3%A9/x.rs");
    assert_eq!(to_path(&u), Some(p));
}

#[test]
fn other_schemes_and_remote_hosts_are_not_paths() {
    assert_eq!(to_path("https://example.com/x"), None);
    assert_eq!(to_path("file://otherhost/x"), None);
    assert_eq!(to_path("file://localhost/x"), Some(PathBuf::from("/x")));
    assert_eq!(
        to_path("file:///x%2"),
        Some(PathBuf::from("/x%2")),
        "a broken escape is kept"
    );
}
