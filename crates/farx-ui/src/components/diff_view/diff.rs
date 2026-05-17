/// A single line in the diff output.
#[derive(Debug, Clone)]
pub(super) enum DiffLine {
    /// Line exists in both files and is identical.
    Same(String),
    /// Line was added (only in right file).
    Added(String),
    /// Line was removed (only in left file).
    Removed(String),
    /// Line was changed (different in left and right).
    Changed(String, String),
}

/// Simple LCS-based diff algorithm.
pub(super) fn compute_diff(left: &[&str], right: &[&str]) -> Vec<DiffLine> {
    let m = left.len();
    let n = right.len();

    // Build LCS table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if left[i - 1] == right[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to produce diff
    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;

    let mut stack = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && left[i - 1] == right[j - 1] {
            stack.push(DiffLine::Same(left[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            stack.push(DiffLine::Added(right[j - 1].to_string()));
            j -= 1;
        } else if i > 0 {
            stack.push(DiffLine::Removed(left[i - 1].to_string()));
            i -= 1;
        }
    }

    // Reverse since we built it backwards
    stack.reverse();

    // Merge adjacent Removed+Added into Changed
    let mut idx = 0;
    while idx < stack.len() {
        if idx + 1 < stack.len() {
            if let (DiffLine::Removed(ref l), DiffLine::Added(ref r)) =
                (&stack[idx], &stack[idx + 1])
            {
                result.push(DiffLine::Changed(l.clone(), r.clone()));
                idx += 2;
                continue;
            }
        }
        result.push(stack[idx].clone());
        idx += 1;
    }

    result
}
