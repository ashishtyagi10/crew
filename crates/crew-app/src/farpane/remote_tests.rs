use super::rclone::RcloneDone;
use super::{FarPane, Location, Side};
use crate::farpane::location::Backend;

#[test]
fn absorb_remotes_populates_the_overlay() {
    let mut f = FarPane::new(std::env::temp_dir());
    f.drive_select = Some(super::DriveSelect::loading(Side::Left));
    let done = RcloneDone {
        code: Some(0),
        stdout: "gdrive:\ndropbox:\n".into(),
        stderr_tail: String::new(),
    };
    f.absorb_remotes(done);
    let ds = f.drive_select.as_ref().unwrap();
    // Local + two remotes
    assert_eq!(ds.options.len(), 3);
}

#[test]
fn choose_remote_reroots_and_lists() {
    let mut f = FarPane::new(std::env::temp_dir());
    f.drive_select = Some(super::DriveSelect {
        side: Side::Left,
        options: vec![super::DriveOption::Remote("gdrive".into())],
        sel: 0,
    });
    let _ = f.choose_drive();
    assert!(f.left.loc.is_remote());
    assert_eq!(f.left.loc.rclone_addr(), "gdrive:");
    assert!(f.pending.is_some(), "re-rooting kicks off a listing");
    assert!(f.drive_select.is_none(), "overlay closes on choose");
}

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

#[test]
fn absorb_simple_success_triggers_relist() {
    let mut f = remote_pane(); // left = gdrive root
    let status = f.absorb_simple(
        Side::Left,
        "deleted",
        RcloneDone {
            code: Some(0),
            stdout: String::new(),
            stderr_tail: String::new(),
        },
    );
    assert!(status.contains("deleted"));
    assert!(
        f.pending.is_some(),
        "a successful mutation re-lists the panel"
    );
}

#[test]
fn absorb_simple_failure_surfaces_stderr_no_relist() {
    let mut f = remote_pane();
    let status = f.absorb_simple(
        Side::Left,
        "deleted",
        RcloneDone {
            code: Some(1),
            stdout: String::new(),
            stderr_tail: "permission denied".into(),
        },
    );
    assert!(status.contains("permission denied"));
    assert!(f.pending.is_none());
}
