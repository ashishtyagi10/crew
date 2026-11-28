//! `sys:read_file` of a picture. Read as text, a PNG is a screenful of
//! garbage; read as a picture, it is the screenshot the user asked about. The
//! call answers with one line (`image: shot.png, 1280x720`) and the loop asks
//! [`attach`] for the picture itself, which rides beside that line in the
//! tool result (`crew_hive::Tools::attachments`).
use crew_hive::provider::{dimensions, omitted, sniff, Image};

/// What a successful picture read starts with, which [`attach`] checks so a
/// repeat's pointer or a text file's contents never attach a picture.
const PREFIX: &str = "image: ";

fn is_image_name(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .iter()
        .any(|e| p.ends_with(e))
}

/// The answer for `path` when it is a picture, or `None` to read it as the
/// text file it may be after all (a `.png` that is not one).
pub(super) fn read(path: &str) -> Option<Result<String, String>> {
    if !is_image_name(path) {
        return None;
    }
    let p = std::path::Path::new(path);
    let name = p.file_name()?.to_string_lossy().into_owned();
    // Named like a picture and far past the limit: say so without reading it.
    if p.metadata().ok()?.len() > 8 << 20 {
        return Some(Ok(omitted(&name, "over the 5 MB limit for one image")));
    }
    let bytes = std::fs::read(p).ok()?;
    sniff(&bytes)?;
    if let Err(note) = Image::from_bytes(&name, &bytes) {
        return Some(Ok(note));
    }
    let size = match dimensions(&bytes) {
        Some((w, h)) => format!("{w}x{h}"),
        None => format!("{} bytes", bytes.len()),
    };
    Some(Ok(format!("{PREFIX}{name}, {size}")))
}

/// The picture a `sys:read_file` call returned, given its `args` and the
/// `output` it answered, or none.
pub(super) fn attach(args: &str, output: &str) -> Vec<Image> {
    if !output.starts_with(PREFIX) {
        return Vec::new();
    }
    let path = serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| v.get("path").and_then(|p| p.as_str()).map(str::to_string));
    let Some(path) = path else {
        return Vec::new();
    };
    // The same confinement the read itself had.
    let Ok(path) = super::sysworkspace::checked(&path, "read_file") else {
        return Vec::new();
    };
    Image::read(std::path::Path::new(&path))
        .into_iter()
        .collect()
}
