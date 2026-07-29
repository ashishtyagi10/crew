// `resolve_md_path`'s absolute/relative resolution logic moved to the shared
// `pathexpand::expand_path` `open_view` now calls — see that module's own
// `keeps_absolute_and_joins_relative` test for the equivalent coverage.

/// `/about` opens what this build changed, not a version number nobody can
/// do anything with. The changelog is compiled in, so an installed binary
/// far from any source tree still answers — and its top entry is guaranteed
/// to be this build's release, because `changelog_covers_the_current_version`
/// fails the build otherwise.
#[test]
fn about_opens_a_file_viewer_pane_rather_than_flashing_a_version() {
    let mut app = crate::app::CrewApp::default();
    let before = app.panes.len();
    app.spawn_about_pane();
    assert_eq!(app.panes.len(), before + 1, "no pane opened");
    assert!(
        matches!(
            app.panes.last().map(|p| &p.content),
            Some(crate::pane::PaneContent::View(_))
        ),
        "/about did not open a file-viewer pane"
    );
    assert!(app.zoomed, "a document pane opens zoomed, like /md");
}

/// The document it opens is the one that ships with the binary.
#[test]
fn the_changelog_is_compiled_in_and_starts_with_this_version() {
    let first = crate::appregister::CHANGELOG
        .lines()
        .find_map(|l| l.strip_prefix("## "))
        .expect("a versioned heading");
    assert_eq!(first.trim(), crate::appregister::VERSION);
}
