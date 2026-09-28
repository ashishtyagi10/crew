use super::*;
use crate::broker::sysgrep::{glob, grep};

fn tree(files: &[(String, String)]) -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("crew-grepfit-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for (rel, text) in files {
        let p = d.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    d
}

const NARROW: &str = "narrow the pattern, or pass \"path\" to search a subdirectory";

/// The first `k` of `want` are what `out` kept, whole and in order — no line
/// cut short, none skipped — and `k` is as many as fit: one more and there
/// would be no room for the closing line.
fn kept_prefix(out: &str, want: &[String]) -> usize {
    let (body, _) = out.rsplit_once('\n').unwrap();
    let kept: Vec<&str> = body.lines().collect();
    let k = kept.len();
    assert_eq!(kept, want[..k], "kept lines are whole and in order");
    assert!(
        body.len() + 1 + want[k].len() + 1 + TAIL > RUN_FIT,
        "{} more bytes of room: the next item would have fit",
        RUN_FIT - body.len()
    );
    k
}

/// 500 hits in five files: as many whole hits as fit under the budget, and one
/// line saying how many hits in how many files were left out.
#[test]
fn many_hits_are_fitted_and_what_was_left_out_is_counted() {
    let files: Vec<(String, String)> = (0..5)
        .map(|f| {
            let text: String = (0..100)
                .map(|i| format!("hit {i:03} of file {f}\n"))
                .collect();
            (format!("f{f}.txt"), text)
        })
        .collect();
    let d = tree(&files);
    let want: Vec<String> = (0..500)
        .map(|n| {
            format!(
                "f{}.txt:{}: hit {:03} of file {}",
                n / 100,
                n % 100 + 1,
                n % 100,
                n / 100
            )
        })
        .collect();
    let out = grep(&serde_json::json!({"pattern": "^hit", "path": d})).unwrap();
    assert!(out.len() <= RUN_FIT, "{} bytes", out.len());
    let k = kept_prefix(&out, &want);
    assert!(k > 100, "only {k} kept");
    let files_left = 5 - k / 100;
    let tail = out.rsplit('\n').next().unwrap();
    assert_eq!(
        tail,
        format!(
            "\u{2026} {} more hits in {files_left} files left out \u{2014} {NARROW}",
            500 - k
        )
    );
}

#[test]
fn a_long_path_list_is_fitted_the_same_way() {
    let files: Vec<(String, String)> = (0..400)
        .map(|i| (format!("src/module_{i:03}.rs"), String::new()))
        .collect();
    let d = tree(&files);
    let want: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
    let out = glob(&serde_json::json!({"pattern": "*.rs", "path": d})).unwrap();
    assert!(out.len() <= RUN_FIT, "{} bytes", out.len());
    let k = kept_prefix(&out, &want);
    let tail = out.rsplit('\n').next().unwrap();
    assert_eq!(
        tail,
        format!("\u{2026} {} more paths left out \u{2014} {NARROW}", 400 - k)
    );
}

/// Up to the budget itself the items come back whole, with no closing line;
/// one byte over, and items come off the end to make room for it.
#[test]
fn the_budget_is_met_exactly_before_anything_is_left_out() {
    let n = RUN_FIT / 10 - 1; // items of 9 bytes and a newline
    let fill = |last: usize| {
        let mut fit = Fit::default();
        fit.file();
        for i in 0..n {
            fit.push(format!("item {i:04}"));
        }
        fit.push("z".repeat(last));
        fit.finish(|left, files| format!("{left} left out of {files}"))
    };
    let exact = RUN_FIT - 10 * n;
    let whole = fill(exact);
    assert_eq!(whole.len(), RUN_FIT);
    assert!(
        whole.ends_with(&format!("\n{}", "z".repeat(exact))),
        "{whole}"
    );
    let cut = fill(exact + 1);
    let k = (RUN_FIT - TAIL) / 10;
    let end = format!("\nitem {:04}\n{} left out of 1", k - 1, n + 1 - k);
    assert!(cut.ends_with(&end), "{}", &cut[cut.len() - 40..]);
}
