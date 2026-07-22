use super::rclone::RcloneDone;
use super::{FarPane, Location, Side};
use crate::farpane::location::Backend;

fn remote_pane() -> FarPane {
    let mut f = FarPane::new(std::env::temp_dir());
    f.left.loc = Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: String::new(),
    };
    f.left.loading = true;
    f
}

#[test]
fn absorb_list_installs_sorted_entries() {
    let mut f = remote_pane();
    let loc = f.left.loc.clone();
    let done = RcloneDone {
        code: Some(0),
        stdout:
            r#"[{"Name":"b.txt","Size":2,"IsDir":false},{"Name":"adir","Size":-1,"IsDir":true}]"#
                .into(),
        stderr_tail: String::new(),
    };
    let status = f.absorb_list(Side::Left, loc, done);
    assert!(!f.left.loading);
    let names: Vec<&str> = f.left.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["adir", "b.txt"]); // remote root: no ".." row
    assert!(status.contains("gdrive:"));
}

#[test]
fn absorb_list_surfaces_rclone_error() {
    let mut f = remote_pane();
    let loc = f.left.loc.clone();
    let done = RcloneDone {
        code: Some(1),
        stdout: String::new(),
        stderr_tail: "auth failed".into(),
    };
    let status = f.absorb_list(Side::Left, loc, done);
    assert!(!f.left.loading);
    assert!(status.contains("auth failed"));
}
