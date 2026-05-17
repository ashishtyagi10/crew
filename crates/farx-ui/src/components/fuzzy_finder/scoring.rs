/// Simple fuzzy scoring: characters from query must appear in order in the text.
/// Higher score for consecutive matches and matches at word boundaries.
pub(super) fn fuzzy_score(text: &str, query: &str) -> i32 {
    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    if query_chars.is_empty() {
        return 1;
    }

    let mut score = 0i32;
    let mut qi = 0;
    let mut prev_match = false;

    for (ti, &tc) in text_chars.iter().enumerate() {
        if qi < query_chars.len() && tc == query_chars[qi] {
            score += 1;
            // Bonus for consecutive matches
            if prev_match {
                score += 2;
            }
            // Bonus for match at start or after separator
            if ti == 0
                || matches!(
                    text_chars.get(ti.wrapping_sub(1)),
                    Some('/' | '\\' | '_' | '-' | '.')
                )
            {
                score += 3;
            }
            qi += 1;
            prev_match = true;
        } else {
            prev_match = false;
        }
    }

    if qi == query_chars.len() {
        score
    } else {
        0 // Not all query chars matched
    }
}
