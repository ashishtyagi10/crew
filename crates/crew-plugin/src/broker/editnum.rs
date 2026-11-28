//! The words of a failed edit in a batch (`sysedit`), apart to keep that file under the line cap.

/// A failure, saying WHICH edit failed and that the file is untouched — but
/// only when there was more than one, since "edit 1 of 1" is noise.
pub(super) fn numbered(total: usize, i: usize, why: &str) -> String {
    match total {
        1 => why.to_string(),
        // After the reason's first line, not under the rows it shows.
        n => {
            let (head, rows) = why.split_once('\n').unwrap_or((why, ""));
            let rows = if rows.is_empty() {
                String::new()
            } else {
                format!("\n{rows}")
            };
            format!(
                "edit {} of {n}: {head} \u{2014} nothing was written{rows}",
                i + 1
            )
        }
    }
}
