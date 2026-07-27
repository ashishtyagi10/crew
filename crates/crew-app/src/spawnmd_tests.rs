use super::*;

#[test]
fn resolve_md_path_keeps_absolute_paths() {
    let cwd = Path::new("/some/cwd");
    assert_eq!(
        resolve_md_path(cwd, "/etc/hosts"),
        PathBuf::from("/etc/hosts")
    );
}

#[test]
fn resolve_md_path_joins_relative_paths_onto_cwd() {
    let cwd = Path::new("/some/cwd");
    assert_eq!(
        resolve_md_path(cwd, "README.md"),
        PathBuf::from("/some/cwd/README.md")
    );
}

/// `/about` opens what this build changed, not a version number nobody can
/// do anything with. The changelog is compiled in, so an installed binary
/// far from any source tree still answers — and its top entry is guaranteed
/// to be this build's release, because `changelog_covers_the_current_version`
/// fails the build otherwise.
#[test]
fn about_opens_a_markdown_pane_rather_than_flashing_a_version() {
    let mut app = crate::app::CrewApp::default();
    let before = app.panes.len();
    app.spawn_about_pane();
    assert_eq!(app.panes.len(), before + 1, "no pane opened");
    assert!(
        matches!(
            app.panes.last().map(|p| &p.content),
            Some(crate::pane::PaneContent::Markdown(_))
        ),
        "/about did not open a markdown pane"
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
