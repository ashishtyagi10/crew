//! Classification helpers for `FileEntry` (executable, archive, image).

pub(super) fn is_executable(entry: &farx_core::FileEntry) -> bool {
    if entry.is_dir {
        return false;
    }
    matches!(
        entry.extension.as_deref(),
        Some("sh" | "bash" | "zsh" | "fish" | "py" | "rb" | "pl")
    )
}

pub(super) fn is_archive(entry: &farx_core::FileEntry) -> bool {
    if entry.is_dir {
        return false;
    }
    matches!(
        entry.extension.as_deref(),
        Some(
            "zip"
                | "tar"
                | "gz"
                | "bz2"
                | "xz"
                | "7z"
                | "rar"
                | "zst"
                | "tgz"
                | "tbz2"
                | "txz"
                | "lz"
                | "lzma"
                | "cab"
                | "iso"
                | "dmg"
                | "jar"
                | "war"
                | "deb"
                | "rpm"
        )
    )
}

pub(super) fn is_image(entry: &farx_core::FileEntry) -> bool {
    if entry.is_dir {
        return false;
    }
    matches!(
        entry.extension.as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "heic")
    )
}
