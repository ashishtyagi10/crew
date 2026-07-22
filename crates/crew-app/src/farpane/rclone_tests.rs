use super::*;
use crate::farpane::location::Backend;

fn gdrive(path: &str) -> Location {
    Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: path.into(),
    }
}

#[test]
fn lsjson_argv_targets_the_address() {
    assert_eq!(
        argv_lsjson(&gdrive("Photos")),
        vec!["lsjson", "gdrive:Photos"]
    );
}

#[test]
fn mkdir_delete_move_copy_argv() {
    let a = gdrive("Photos/a.txt");
    let b = gdrive("Backup/a.txt");
    assert_eq!(argv_mkdir(&gdrive("New")), vec!["mkdir", "gdrive:New"]);
    // file delete uses `deletefile`; dir delete uses `purge`
    assert_eq!(
        argv_delete(&a, false),
        vec!["deletefile", "gdrive:Photos/a.txt"]
    );
    assert_eq!(
        argv_delete(&gdrive("Photos"), true),
        vec!["purge", "gdrive:Photos"]
    );
    // file copy/move use the *to variants; dirs use plain copy/move
    assert_eq!(
        argv_copy(&a, &b, false),
        vec!["copyto", "gdrive:Photos/a.txt", "gdrive:Backup/a.txt"]
    );
    assert_eq!(
        argv_move(&a, &b, false),
        vec!["moveto", "gdrive:Photos/a.txt", "gdrive:Backup/a.txt"]
    );
    let da = gdrive("Photos/sub");
    let db = gdrive("Backup/sub");
    assert_eq!(
        argv_copy(&da, &db, true),
        vec!["copy", "gdrive:Photos/sub", "gdrive:Backup/sub"]
    );
}

#[test]
fn parse_lsjson_maps_fields_and_sorts_with_parent_row() {
    // rclone lsjson emits an array of {Name, Size, IsDir, ...}
    let json = r#"[
        {"Name":"small.txt","Size":1,"IsDir":false},
        {"Name":"zdir","Size":-1,"IsDir":true},
        {"Name":"big.txt","Size":500,"IsDir":false},
        {"Name":"adir","Size":-1,"IsDir":true}
    ]"#;
    let loc = Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: "Photos".into(),
    };
    let entries = parse_lsjson(json, &loc).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["..", "adir", "zdir", "big.txt", "small.txt"]);
    assert!(entries[0].is_parent);
    assert_eq!(entries[3].size, 500);
    assert_eq!(entries[1].size, 0, "directories carry no size");
}

#[test]
fn parse_lsjson_at_root_has_no_parent_row() {
    let loc = Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: String::new(),
    };
    let entries = parse_lsjson("[]", &loc).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn parse_lsjson_rejects_garbage() {
    let loc = Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: String::new(),
    };
    assert!(parse_lsjson("not json", &loc).is_err());
}
