use std::path::Path;

pub fn get_disk_space(_path: &Path) -> (Option<u64>, Option<u64>) {
    // Platform-specific disk space query
    #[cfg(unix)]
    {
        use std::ffi::CString;
        let c_path = CString::new(_path.to_string_lossy().as_bytes()).ok();
        if let Some(c_path) = c_path {
            unsafe {
                let mut stat: libc::statvfs = std::mem::zeroed();
                if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                    let free = stat.f_bavail as u64 * stat.f_frsize;
                    let total = stat.f_blocks as u64 * stat.f_frsize;
                    return (Some(free), Some(total));
                }
            }
        }
        (None, None)
    }
    #[cfg(not(unix))]
    {
        (None, None)
    }
}

pub fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Check if a filename has an image extension.
pub fn is_image_ext(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".bmp")
        || lower.ends_with(".webp")
        || lower.ends_with(".ico")
        || lower.ends_with(".tiff")
        || lower.ends_with(".tif")
}
