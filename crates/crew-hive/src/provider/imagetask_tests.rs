use super::*;
use crate::provider::image::fake_png;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-imagetask-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn put(d: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = d.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn a_pasted_temp_path_attaches_and_the_text_stays() {
    let d = dir("paste");
    let p = put(&d, "crew-paste-1700000000000.png", &fake_png(8, 8));
    let ask = format!("why is this button clipped? {}", p.display());
    let got = from_task(&ask, Path::new("/nonexistent"));
    assert_eq!(got.images.len(), 1);
    assert_eq!(got.images[0].name, "crew-paste-1700000000000.png");
    assert!(got.notes.is_empty());
}

#[test]
fn a_relative_path_resolves_against_the_workspace() {
    let d = dir("rel");
    put(&d, "mock.PNG", &fake_png(1, 1));
    let got = from_task("match \"mock.PNG\", please.", &d);
    assert_eq!(got.images.len(), 1);
}

#[test]
fn a_path_that_names_no_file_attaches_nothing_and_says_nothing() {
    let d = dir("missing");
    let got = from_task("see /no/such/shot.png and also ghost.jpg", &d);
    assert!(got.images.is_empty());
    assert!(got.notes.is_empty());
}

#[test]
fn a_directory_or_a_non_image_is_not_attached() {
    let d = dir("kinds");
    std::fs::create_dir_all(d.join("dir.png")).unwrap();
    put(&d, "notes.txt", b"hi");
    let got = from_task("dir.png notes.txt", &d);
    assert!(got.images.is_empty() && got.notes.is_empty());
}

#[test]
fn the_same_file_named_twice_is_attached_once() {
    let d = dir("dup");
    put(&d, "a.png", &fake_png(1, 1));
    assert_eq!(from_task("a.png ./a.png a.png", &d).images.len(), 1);
}

#[test]
fn the_sixth_image_gets_a_note() {
    let d = dir("six");
    let names: Vec<String> = (0..6).map(|i| format!("s{i}.png")).collect();
    for n in &names {
        put(&d, n, &fake_png(1, 1));
    }
    let got = from_task(&names.join(" "), &d);
    assert_eq!(got.images.len(), 5);
    assert_eq!(
        got.notes,
        ["[image omitted: s5.png, only the first 5 images are attached]"]
    );
}

#[test]
fn an_image_over_5_mb_gets_a_note() {
    let d = dir("big");
    let mut big = fake_png(1, 1);
    big.resize(5 * 1024 * 1024, 0);
    put(&d, "huge.png", &big);
    let got = from_task("huge.png", &d);
    assert!(got.images.is_empty());
    assert!(
        got.notes[0].starts_with("[image omitted: huge.png, "),
        "{:?}",
        got.notes
    );
}

#[test]
fn a_file_that_is_not_the_image_its_name_says_gets_a_note() {
    let d = dir("fake");
    put(&d, "fake.png", b"this is text");
    let got = from_task("fake.png", &d);
    assert!(got.images.is_empty());
    assert!(got.notes[0].contains("not a PNG"), "{:?}", got.notes);
}
