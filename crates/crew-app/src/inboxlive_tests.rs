//! The mail reader against the user's own mail (goal
//! 2026-10-21-mail-looks-right, the loop's last check), with their leave.
//! Two `#[ignore]`d steps, both run by hand, both outside the repo:
//!
//! `CREW_LIVE_GRANT=<gmail.json> CREW_LIVE_DIR=<dir> cargo test -p crew-app
//! --bin crew inbox_live_fetch -- --ignored` saves recent threads as Gmail
//! sends them (`format=full`) to `<dir>/raw/`. Nothing is marked read, and
//! no picture is fetched (that would tell a sender the mail was opened).
//!
//! `CREW_LIVE_DIR=<dir> cargo test … inbox_live_read -- --ignored` reads
//! each saved thread as the reader does and writes `<dir>/md/<id>.md`, so a
//! change to the reader is judged against the same mail every round.
use std::path::PathBuf;

use crate::inbox::api::Gmail;

fn dir() -> Option<PathBuf> {
    std::env::var_os("CREW_LIVE_DIR").map(PathBuf::from)
}

#[test]
#[ignore = "reads the user's own Gmail; needs CREW_LIVE_GRANT and CREW_LIVE_DIR"]
fn inbox_live_fetch() {
    let (Some(dir), Some(grant)) = (dir(), std::env::var_os("CREW_LIVE_GRANT")) else {
        return eprintln!("CREW_LIVE_GRANT and CREW_LIVE_DIR unset: nothing fetched");
    };
    let grant = crate::inbox::grant::load_from(std::path::Path::new(&grant)).expect("a grant");
    let mut g = Gmail::new(grant).expect("a client");
    let raw = dir.join("raw");
    std::fs::create_dir_all(&raw).unwrap();
    let mut ids: Vec<String> = Vec::new();
    for (q, n) in [
        ("in:inbox", 30),
        ("category:promotions newer_than:60d", 10),
        ("category:updates newer_than:60d", 10),
    ] {
        let list = format!("/threads?maxResults={n}&q={}", crate::inbox::query::enc(q));
        let (found, _) = crate::inbox::wire::thread_ids(&g.get(&list).expect("a list"));
        for id in found {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    let mut saved = 0;
    for id in &ids {
        if let Ok(v) = g.get(&format!("/threads/{id}?format=full")) {
            std::fs::write(raw.join(format!("{id}.json")), v.to_string()).unwrap();
            saved += 1;
        }
    }
    eprintln!("saved {saved} of {} threads", ids.len());
}

#[test]
#[ignore = "reads the user's saved mail; needs CREW_LIVE_DIR"]
fn inbox_live_read() {
    let Some(dir) = dir() else {
        return eprintln!("CREW_LIVE_DIR unset: nothing read");
    };
    let out = dir.join("md");
    std::fs::create_dir_all(&out).unwrap();
    let mut n = 0;
    for entry in std::fs::read_dir(dir.join("raw")).unwrap().flatten() {
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
        let mut md = String::new();
        for m in v["messages"].as_array().cloned().unwrap_or_default() {
            let (rich, files) = crate::inbox::body::text(&m);
            let rich = match rich.with_web.clone() {
                Some(w) => *w,
                None => rich,
            };
            md.push_str(&format!(
                "===== {} | {} | files {files:?}\n\n{}\n",
                crate::inbox::wire::header(&m, "From"),
                crate::inbox::wire::header(&m, "Subject"),
                rich.body
            ));
            if let Some(f) = &rich.foot {
                md.push_str(&format!("\n===== small print\n\n{f}\n"));
            }
            if let Some(q) = &rich.quote {
                md.push_str(&format!("\n===== quote ({} lines)\n", q.lines().count()));
            }
        }
        let id = entry
            .path()
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        std::fs::write(out.join(format!("{id}.md")), md).unwrap();
        n += 1;
    }
    eprintln!("read {n} threads");
}

/// `CREW_LIVE_DIR=<dir> CREW_SHOT_DIR=<dir> CREW_LIVE_SHOTS=<id,id…> cargo
/// test … inbox_live_shot -- --ignored`: the saved threads named, drawn in
/// the reader on both themes, top and further down, their pictures stood
/// in for by boxes of their shape (none is fetched).
#[test]
#[ignore = "draws the user's saved mail; needs CREW_LIVE_DIR, CREW_LIVE_SHOTS and a GPU"]
fn inbox_live_shot() {
    let (Some(dir), Some(ids)) = (dir(), std::env::var("CREW_LIVE_SHOTS").ok()) else {
        return eprintln!("CREW_LIVE_DIR and CREW_LIVE_SHOTS unset: nothing drawn");
    };
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in ids.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let path = std::fs::read_dir(dir.join("raw"))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(id))
            });
        let Some(path) = path else { continue };
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut p = crate::inboxshot_tests::ready();
        let msgs = v["messages"].as_array().cloned().unwrap_or_default();
        let mails: Vec<crate::inbox::api::Mail> = msgs
            .iter()
            .map(|m| {
                let (rich, files) = crate::inbox::body::text(m);
                let srcs = crate::inbox::pics::web_pics(&rich).chain(rich.pics.iter());
                for (n, src) in srcs.enumerate() {
                    let (w, h) = [(600, 300), (300, 300), (160, 48), (400, 300)][n % 4];
                    let bm = crate::viewpane::bitmap::Bitmap {
                        w: w / 4,
                        h: h / 4,
                        px: vec![[150, 160, 200, 255]; (w / 4 * h / 4) as usize],
                        src: (w, h),
                    };
                    let key = crate::inbox::pics::key(m["id"].as_str().unwrap_or(""), src);
                    p.pics.set(key, Some(bm));
                }
                crate::inbox::api::Mail {
                    id: m["id"].as_str().unwrap_or_default().into(),
                    from: crate::inbox::wire::header(m, "From"),
                    date: crate::inbox::wire::header(m, "Date"),
                    text: rich.whole(),
                    files,
                    rich,
                    ..Default::default()
                }
            })
            .collect();
        let thread = crate::inbox::wire::Thread {
            subject: msgs
                .first()
                .map(|m| crate::inbox::wire::header(m, "Subject"))
                .unwrap_or_default(),
            ..Default::default()
        };
        for depth in [0, 45] {
            p.reading = Some(crate::inbox::Reading {
                mails: Some(mails.clone()),
                web: true,
                scroll: depth,
                ..crate::inbox::Reading::new(thread.clone())
            });
            p.sync_sizes();
            for theme in [
                crew_theme::ThemeId::PaperLight,
                crew_theme::ThemeId::GlassNight,
            ] {
                crew_theme::set_theme(theme);
                crate::palette::set_accent(crew_theme::theme().accent_default);
                let tag = format!("live-{id}-{depth}-{}", format!("{theme:?}").to_lowercase());
                let legend = p.legend();
                crate::shotgpu_tests::shot_at(&tag, 760, 900, 13.0, &legend, |c, r, a| {
                    p.art(c, r, a)
                });
            }
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
