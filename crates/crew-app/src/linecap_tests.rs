use super::*;

#[test]
fn no_file_grows_past_its_recorded_length_or_crosses_the_cap() {
    let debt: std::collections::HashMap<&str, usize> = debt().into_iter().collect();
    let (mut grew, mut crossed, mut finished) = (Vec::new(), Vec::new(), Vec::new());
    let exempt: std::collections::HashSet<&str> = EXEMPT.iter().map(|(f, _)| *f).collect();
    for (name, len) in sources() {
        if exempt.contains(name.as_str()) {
            continue;
        }
        match debt.get(name.as_str()) {
            Some(&was) if len > was => grew.push(format!("{name}: {was} \u{2192} {len}")),
            Some(&was) if len <= LINE_CAP => finished.push(format!("{name}: {was} \u{2192} {len}")),
            Some(_) => {}
            None if len > LINE_CAP => crossed.push(format!("{name}: {len}")),
            None => {}
        }
    }
    assert!(
        grew.is_empty(),
        "already over the {LINE_CAP}-line cap, and now LONGER:\n  {}",
        grew.join("\n  ")
    );
    assert!(
        crossed.is_empty(),
        "crossed the {LINE_CAP}-line cap \u{2014} split along a responsibility \
         boundary rather than adding a row to the debt:\n  {}",
        crossed.join("\n  ")
    );
    assert!(
        finished.is_empty(),
        "under the cap now \u{2014} delete these rows from line-cap-debt.txt so \
         they can never drift back:\n  {}",
        finished.join("\n  ")
    );
}

/// Every row names a file that exists. A rename leaves its row behind, and a
/// row nothing matches is a silently unenforced entry.
#[test]
fn every_recorded_file_still_exists() {
    let present: std::collections::HashSet<String> =
        sources().into_iter().map(|(n, _)| n).collect();
    let stale: Vec<&str> = debt()
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| !present.contains(*n))
        .collect();
    assert!(stale.is_empty(), "rows for files that are gone: {stale:?}");
}

/// An exemption states a reason, and names a file that is really there.
#[test]
fn every_exemption_is_justified_and_real() {
    let present: std::collections::HashSet<String> =
        sources().into_iter().map(|(n, _)| n).collect();
    for (file, why) in EXEMPT {
        assert!(present.contains(*file), "exempt file is gone: {file}");
        assert!(
            why.len() > 20,
            "exemption needs a reason, not a shrug: {file}"
        );
    }
}
