use super::*;
use std::path::Path;

#[test]
fn local_round_trips_a_path() {
    let loc = Location::local(Path::new("/home/x/proj"));
    assert!(!loc.is_remote());
    assert_eq!(loc.local_path().unwrap(), Path::new("/home/x/proj"));
    assert_eq!(loc.rclone_addr(), "/home/x/proj");
}

#[test]
fn remote_addr_is_remote_colon_path() {
    let root = Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: String::new(),
    };
    assert!(root.is_remote());
    assert_eq!(root.rclone_addr(), "gdrive:");
    assert!(root.local_path().is_none());
    let photos = root.child("Photos");
    assert_eq!(photos.rclone_addr(), "gdrive:Photos");
    assert_eq!(photos.child("2024").rclone_addr(), "gdrive:Photos/2024");
}

#[test]
fn remote_parent_ascends_and_stops_at_root() {
    let deep = Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: "Photos/2024".into(),
    };
    assert!(deep.has_parent());
    let up = deep.parent().unwrap();
    assert_eq!(up.rclone_addr(), "gdrive:Photos");
    let root = up.parent().unwrap();
    assert_eq!(root.rclone_addr(), "gdrive:");
    assert!(!root.has_parent());
    assert!(root.parent().is_none());
}

#[test]
fn local_parent_matches_path_parent() {
    let loc = Location::local(Path::new("/a/b"));
    assert!(loc.has_parent());
    assert_eq!(loc.parent().unwrap().local_path().unwrap(), Path::new("/a"));
}
