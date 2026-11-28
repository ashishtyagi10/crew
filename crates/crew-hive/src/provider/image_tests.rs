use super::*;

#[test]
fn the_first_bytes_pick_the_type_not_the_extension() {
    assert_eq!(sniff(&fake_png(1, 1)), Some("image/png"));
    assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some("image/jpeg"));
    assert_eq!(sniff(b"GIF89a.."), Some("image/gif"));
    assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
    assert_eq!(sniff(b"plain text"), None);
    // A JPEG saved as shot.png is sent as a JPEG.
    let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 0];
    assert_eq!(
        Image::from_bytes("shot.png", &jpeg).unwrap().media_type,
        "image/jpeg"
    );
}

#[test]
fn a_header_gives_the_size_without_decoding() {
    assert_eq!(dimensions(&fake_png(1280, 720)), Some((1280, 720)));
    assert_eq!(dimensions(b"GIF89a\x10\x00\x20\x00"), Some((16, 32)));
    assert_eq!(dimensions(b"plain"), None);
}

#[test]
fn a_jpeg_size_is_read_from_its_frame_segment() {
    // SOI, APP0 (len 4), SOF0: precision, height 0x0102, width 0x0304.
    let b = [
        0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xC0, 0, 11, 8, 1, 2, 3, 4, 1, 0, 0,
    ];
    assert_eq!(dimensions(&b), Some((0x0304, 0x0102)));
}

#[test]
fn a_file_past_the_limit_is_a_note_not_an_image() {
    let mut big = fake_png(1, 1);
    big.resize(4 * 1024 * 1024, 0); // 5.33 MB once base64'd
    let note = Image::from_bytes("big.png", &big).unwrap_err();
    assert!(note.starts_with("[image omitted: big.png, "), "{note}");
    assert!(note.contains("5 MB"), "{note}");
    let mut ok = fake_png(1, 1);
    ok.resize(3 * 1024 * 1024, 0); // 4 MB encoded
    assert!(Image::from_bytes("ok.png", &ok).is_ok());
}

#[test]
fn not_an_image_is_a_note() {
    let note = Image::from_bytes("a.png", b"hello").unwrap_err();
    assert!(note.contains("not a PNG"), "{note}");
}

#[test]
fn debug_never_prints_the_payload() {
    let img = Image::from_bytes("a.png", &fake_png(2, 2)).unwrap();
    let shown = format!("{img:?}");
    assert!(!shown.contains(img.data()), "{shown}");
    assert!(shown.contains("a.png"), "{shown}");
}

#[test]
fn the_data_url_is_the_openai_spelling() {
    let img = Image::from_bytes("a.png", &fake_png(2, 2)).unwrap();
    assert!(img.data_url().starts_with("data:image/png;base64,"));
    assert!(img.data_url().ends_with(img.data()));
}
