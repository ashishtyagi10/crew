//! Sixel size and pixels: the palette, both colour definitions, repeats,
//! bands, transparency, and that nonsense neither panics nor allocates.
use super::*;

/// The pixel at `(x, y)` of a decoded picture.
fn px(img: &(u32, u32, Vec<u8>), x: u32, y: u32) -> [u8; 4] {
    let o = ((y * img.0 + x) * 4) as usize;
    img.2[o..o + 4].try_into().unwrap()
}

fn dec(body: &str) -> (u32, u32, Vec<u8>) {
    decode(body.as_bytes()).expect("decodes")
}

#[test]
fn the_default_palette_is_the_vt340s() {
    let img = dec("q#1~#2~#15~");
    assert_eq!((img.0, img.1), (3, 6));
    assert_eq!(
        px(&img, 0, 0),
        [0x33, 0x33, 0xcc, 255],
        "register 1 is blue"
    );
    assert_eq!(px(&img, 1, 5), [0xcc, 0x21, 0x21, 255], "register 2 is red");
    assert_eq!(px(&img, 2, 3), [0xcc, 0xcc, 0xcc, 255], "register 15");
    assert_eq!(px(&dec("q~"), 0, 0), [0, 0, 0, 255], "register 0 is black");
}

#[test]
fn rgb_definitions_are_percent() {
    let img = dec("q#9;2;100;0;50!2~");
    assert_eq!(px(&img, 1, 0), [255, 0, 128, 255]);
    let img = dec("q#9;2;250;0;0~");
    assert_eq!(px(&img, 0, 0), [255, 0, 0, 255], "past 100% is clamped");
}

/// DEC's wheel: blue at 0 degrees, red at 120, green at 240.
#[test]
fn hls_definitions_start_at_blue() {
    let img = dec("q#1;1;0;50;100~#2;1;120;50;100~#3;1;240;50;100~#4;1;120;100;0~");
    assert_eq!(px(&img, 0, 0), [0, 0, 255, 255], "0 is blue");
    assert_eq!(px(&img, 1, 0), [255, 0, 0, 255], "120 is red");
    assert_eq!(px(&img, 2, 0), [0, 255, 0, 255], "240 is green");
    assert_eq!(px(&img, 3, 0), [255, 255, 255, 255], "L100 is white");
}

#[test]
fn a_repeat_widens_a_column_and_bits_run_top_down() {
    // `@` is bit 0 only (top pixel), `A` bit 1 (second pixel).
    let img = dec("q#1!3@!2A");
    assert_eq!((img.0, img.1), (5, 6));
    for x in 0..3 {
        assert_eq!(px(&img, x, 0)[3], 255);
        assert_eq!(px(&img, x, 1)[3], 0);
    }
    assert_eq!(px(&img, 3, 0)[3], 0);
    assert_eq!(px(&img, 4, 1)[3], 255);
}

#[test]
fn carriage_return_overprints_and_dash_starts_a_band() {
    // `$` goes back: the second colour lands over the first.
    let img = dec("q#1~$#2@");
    assert_eq!(px(&img, 0, 0), [0xcc, 0x21, 0x21, 255]);
    assert_eq!(px(&img, 0, 1), [0x33, 0x33, 0xcc, 255]);
    let img = dec("q#1~-#2~");
    assert_eq!((img.0, img.1), (1, 12));
    assert_eq!(
        px(&img, 0, 5),
        [0x33, 0x33, 0xcc, 255],
        "band one's last row"
    );
    assert_eq!(px(&img, 0, 6), [0xcc, 0x21, 0x21, 255]);
    assert_eq!(
        px(&img, 0, 11),
        [0xcc, 0x21, 0x21, 255],
        "band two's last row"
    );
}

/// A definition with fewer than five numbers selects and defines nothing.
#[test]
fn a_short_definition_only_selects() {
    let img = dec("q#1;2;100~");
    assert_eq!(px(&img, 0, 0), [0x33, 0x33, 0xcc, 255]);
}

/// Unset pixels are transparent for every P2 — see [`decode`]'s doc.
#[test]
fn unset_pixels_are_transparent_whatever_p2_says() {
    for p2 in ["0", "1", "2"] {
        let img = dec(&format!("0;{p2};0q\"1;1;4;6#1@"));
        assert_eq!((img.0, img.1), (4, 6), "P2={p2}");
        assert_eq!(px(&img, 0, 0), [0x33, 0x33, 0xcc, 255]);
        assert_eq!(px(&img, 3, 0), [0, 0, 0, 0], "beyond the data");
        assert_eq!(px(&img, 0, 5), [0, 0, 0, 0], "below the bits");
    }
}

#[test]
fn raster_attributes_give_the_size_when_present() {
    assert_eq!(size(b"q\"1;1;30;12#1~"), Some((30, 12)));
    // A raster that says nothing falls back to the data.
    assert_eq!(size(b"q\"1;1;0;0#1!10~$!5~-~"), Some((10, 12)));
    // Where the raster is smaller than the data, the data is clipped to it.
    let img = dec("q\"1;1;2;6#1!9~");
    assert_eq!((img.0, img.1, img.2.len()), (2, 6, 48));
}

#[test]
fn the_extent_is_scanned_without_a_raster() {
    assert_eq!(size(b"q#1!10~$!5~-~"), Some((10, 12)));
    assert_eq!(size(b"q~~~"), Some((3, 6)));
    assert_eq!(
        size(b"q~-"),
        Some((1, 6)),
        "an empty trailing band is not a row"
    );
    assert_eq!(size(b"q#1;2;1;2;3-"), None, "no pixels, no picture");
}

#[test]
fn absurd_sizes_are_clamped_not_believed() {
    assert_eq!(size(b"q!999999999~"), Some((MAX_DIM, 6)));
    assert!(decode(b"q!999999999~").is_some_and(|i| i.2.len() == 240_000));
    assert_eq!(size(b"q\"1;1;99999;99999~"), None, "100M pixels");
    assert_eq!(size(b"q\"1;1;99999;10~"), Some((MAX_DIM, 10)));
    assert_eq!(size(b"q!99999999999999999999999~"), Some((MAX_DIM, 6)));
}

#[test]
fn not_a_sixel_header_is_not_a_picture() {
    for b in [&b""[..], b"~~", b"+q544e", b"1;2;3~", b"$q~", b"0;1;0"] {
        assert_eq!((size(b), decode(b)), (None, None), "{b:?}");
    }
}

#[test]
fn garbage_never_panics_and_always_matches_its_own_size() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    for n in 0..3000 {
        let mut body = b"1;1;0q".to_vec();
        for _ in 0..(n % 200) {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let alphabet = b"#!\"$-;0123456789?@~}\x1b\x00\xff";
            body.push(alphabet[(seed >> 33) as usize % alphabet.len()]);
        }
        if let Some((w, h)) = size(&body) {
            let img = decode(&body).expect("sized, so decodes");
            assert_eq!(img.2.len(), (w * h * 4) as usize);
        }
    }
    // A register number far past 255 and a definition cut short.
    assert!(decode(b"q#99999999;2;1;2;3~#5;2;1").is_some());
}
