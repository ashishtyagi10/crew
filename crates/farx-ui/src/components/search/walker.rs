use super::action::SearchResult;
use super::glob::matches_glob;
use std::path::Path;

pub(crate) fn search_recursive(
    dir: &Path,
    pattern: &str,
    content_query: &str,
    results: &mut Vec<SearchResult>,
    limit: usize,
) {
    if results.len() >= limit {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries {
        if results.len() >= limit {
            return;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = metadata.is_dir();

        // Check name match
        if matches_glob(&name, pattern) {
            // If content query is set and it's a file, check content and collect matching lines
            if !content_query.is_empty() && !is_dir {
                // Skip large files (> 10MB) for content search
                if metadata.len() <= 10 * 1024 * 1024 {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let query_lower = content_query.to_lowercase();
                        let matching_lines: Vec<(usize, String)> = content
                            .lines()
                            .enumerate()
                            .filter(|(_, line)| line.to_lowercase().contains(&query_lower))
                            .take(5) // max 5 matching lines per file
                            .map(|(num, line)| {
                                let trimmed: String = line.chars().take(200).collect();
                                (num + 1, trimmed)
                            })
                            .collect();
                        if !matching_lines.is_empty() {
                            results.push(SearchResult {
                                path: path.clone(),
                                name: name.clone(),
                                is_dir,
                                size: metadata.len(),
                                matching_lines,
                            });
                        }
                    }
                }
            } else {
                // No content filter or is a directory
                results.push(SearchResult {
                    path: path.clone(),
                    name: name.clone(),
                    is_dir,
                    size: if is_dir { 0 } else { metadata.len() },
                    matching_lines: Vec::new(),
                });
            }
        }

        // Recurse into directories
        if is_dir && !name.starts_with('.') {
            search_recursive(&path, pattern, content_query, results, limit);
        }
    }
}
