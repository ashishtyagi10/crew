//! What a document's `![alt](src)` is allowed to mean, and what it is not.
use super::*;

fn png(path: &Path, w: u32, h: u32) {
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([(x * 4) as u8, (y * 4) as u8, 90, 255])
    });
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(path, image::ImageFormat::Png)
        .expect("write");
}

fn tmp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(name);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

/// A relative source resolves against the DOCUMENT, not the process's working
/// directory — a README opened from anywhere names its images the same way.
#[test]
fn a_relative_source_resolves_against_the_document() {
    let dir = tmp_dir("imgcache-rel");
    let doc = dir.join("README.md");
    std::fs::write(&doc, "# hi").expect("write");
    let img = dir.join("logo.png");
    png(&img, 8, 8);
    assert_eq!(resolve("logo.png", &doc), Some(img.clone()));
    assert_eq!(resolve(&img.to_string_lossy(), &doc), Some(img));
    assert_eq!(resolve("nope.png", &doc), None, "a path with no file");
}

/// A remote image is a network fetch, and a terminal must not make one on its
/// own because a document said so. It stays alt text.
#[test]
fn a_remote_source_is_never_fetched() {
    let doc = std::env::temp_dir().join("README.md");
    assert_eq!(resolve("https://example.invalid/a.png", &doc), None);
    assert_eq!(resolve("http://example.invalid/a.png", &doc), None);
    assert_eq!(resolve("data:image/png;base64,AAAA", &doc), None);
}

/// The frame never reads a file: the first ask starts a worker and returns
/// nothing, and a later ask has the picture.
#[test]
fn the_first_ask_starts_a_read_and_a_later_one_has_it() {
    let dir = tmp_dir("imgcache-load");
    let img = dir.join("shot.png");
    png(&img, 24, 12);
    assert_eq!(get(&img), None, "the frame must not block on I/O");
    let mut got = None;
    for _ in 0..200 {
        if let Some(bm) = get(&img) {
            got = Some(bm);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let bm = got.expect("the worker never came back");
    assert_eq!(bm.src, (24, 12), "the picture that was actually named");
    assert!(!pending(&img), "…and nothing is still out for it");
}

/// A path that is not a picture must fail once and stay failed, or every
/// frame forever spawns a worker for it.
#[test]
fn something_that_is_not_a_picture_fails_once() {
    let dir = tmp_dir("imgcache-bad");
    let bad = dir.join("notes.txt");
    std::fs::write(&bad, "this is not a png").expect("write");
    assert_eq!(get(&bad), None);
    for _ in 0..200 {
        if !pending(&bad) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(get(&bad), None);
    assert!(
        !pending(&bad),
        "a failure must not be retried on every frame"
    );
}
