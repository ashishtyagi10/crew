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
fn remote_mkdir_starts_a_simple_op() {
    let mut f = remote_pane(); // left active, remote
    f.active = Side::Left;
    let action = crate::farpane::fileops::make_dir(&mut f, "New");
    assert!(matches!(action, crate::farpane::keys::FarAction::Status(_)));
    assert!(f.pending.is_some());
}

#[test]
fn copy_local_to_remote_is_async() {
    let mut f = FarPane::new(std::env::temp_dir()); // left local
    f.right.loc = Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: String::new(),
    };
    f.active = Side::Left;
    // put a fake selected file in the local panel
    f.left.entries = vec![crate::farpane::Entry {
        name: "a.txt".into(),
        is_dir: false,
        is_parent: false,
        size: 1,
    }];
    f.left.sel = 0;
    let action = crate::farpane::fileops::copy(&mut f);
    assert!(matches!(action, crate::farpane::keys::FarAction::Status(_)));
    assert!(
        f.pending.is_some(),
        "a transfer touching a remote runs on rclone"
    );
}

#[test]
fn absorb_transfer_reloads_local_right_when_remote_is_left() {
    // Regression for the review bug: the old loop `break`d the instant it
    // found a remote side, so with Left=remote/Right=local the local Right
    // panel was never reloaded after a successful transfer.
    let dir = std::env::temp_dir().join(format!(
        "far_absorb_transfer_regress_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("new.txt"), b"hi").unwrap();

    let mut f = FarPane::new(std::env::temp_dir());
    f.left.loc = Location {
        backend: Backend::Rclone {
            remote: "gdrive".into(),
        },
        path: String::new(),
    };
    f.right.loc = crate::farpane::location::Location::local(&dir);
    f.right.entries = vec![]; // deliberately stale

    let status = f.absorb_transfer(
        "copied",
        RcloneDone {
            code: Some(0),
            stdout: String::new(),
            stderr_tail: String::new(),
        },
    );

    assert!(status.contains("copied"));
    assert!(
        !f.right.entries.is_empty(),
        "local Right panel must reload even though Left is remote"
    );
    assert!(
        f.pending.is_some(),
        "remote Left panel must still kick off a listing"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn absorb_download_opens_temp_and_registers_watch() {
    let mut f = remote_pane();
    let remote = f.left.loc.child("notes.txt");
    let temp = std::env::temp_dir().join("far-drive-test-notes.txt");
    std::fs::write(&temp, b"hi").unwrap();
    let action = f.absorb_download(
        remote.clone(),
        temp.clone(),
        RcloneDone {
            code: Some(0),
            stdout: String::new(),
            stderr_tail: String::new(),
        },
    );
    assert!(matches!(action, crate::farpane::keys::FarAction::Open(ref p) if p == &temp));
    assert_eq!(f.watches.len(), 1);
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn absorb_download_failure_surfaces_stderr_no_watch() {
    let mut f = remote_pane();
    let remote = f.left.loc.child("notes.txt");
    let temp = std::env::temp_dir().join("far-drive-test-notes-fail.txt");
    let action = f.absorb_download(
        remote,
        temp,
        RcloneDone {
            code: Some(1),
            stdout: String::new(),
            stderr_tail: "auth failed".into(),
        },
    );
    assert!(
        matches!(action, crate::farpane::keys::FarAction::Status(ref s) if s.contains("auth failed"))
    );
    assert!(f.watches.is_empty());
}

#[test]
fn begin_download_starts_a_pending_transfer() {
    let mut f = remote_pane(); // left = gdrive root, active by default
    f.left.entries = vec![crate::farpane::Entry {
        name: "notes.txt".into(),
        is_dir: false,
        is_parent: false,
        size: 3,
    }];
    f.left.sel = 0;
    let action = f.begin_download("notes.txt");
    assert!(matches!(action, crate::farpane::keys::FarAction::Status(_)));
    assert!(f.pending.is_some(), "download runs on rclone");
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
