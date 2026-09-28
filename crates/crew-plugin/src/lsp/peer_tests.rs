#![cfg(unix)]
//! A scripted language server for the host's tests. `sh` replays canned
//! frames (the `initialize` answer, a reply for every request after it, and
//! whatever publishes a test scripts, each after its own pause) and then
//! copies everything crew sent it into a file the test reads back.
//!
//! Publishes are paced rather than sent up front because the host drains
//! what is queued before each call: one sent with the handshake is already
//! history by the time a call waits for the server's word.
use super::*;
use crew_lsp::framing::{decode, encode};
use serde_json::{json, Value};

/// rust-analyzer's own `textDocumentSync`: incremental, saves without text.
pub(super) fn rust_analyzer() -> Value {
    json!({"textDocumentSync": {"openClose": true, "change": 2, "save": {}}})
}

pub(super) struct Peer {
    pub(super) host: LspHost,
    dir: PathBuf,
    pub(super) file: PathBuf,
    heard: PathBuf,
    hovers: usize,
}

impl Peer {
    pub(super) fn new(tag: &str, capabilities: Value) -> Self {
        Self::scripted(tag, capabilities, &[])
    }

    /// A peer that, after the handshake, sleeps each `(seconds, error)`
    /// pause and then publishes for `src/lib.rs`: nothing wrong, or `error`.
    pub(super) fn scripted(tag: &str, capabilities: Value, steps: &[(f32, Option<&str>)]) -> Self {
        let dir = std::env::temp_dir().join(format!("crew-lspfresh-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"p\"\n").unwrap();
        let file = dir.join("src/lib.rs");
        std::fs::write(&file, "fn a() {}\n").unwrap();
        let mut canned = encode(&json!({"jsonrpc": "2.0", "id": 1,
            "result": {"capabilities": capabilities}}));
        for id in 2..20 {
            canned.extend(encode(&json!({"jsonrpc": "2.0", "id": id, "result": null})));
        }
        let cat = |name: String, bytes: Vec<u8>| {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            format!("cat '{}'", path.display())
        };
        let mut script = vec![cat("canned".into(), canned)];
        for (i, (pause, error)) in steps.iter().enumerate() {
            let publish = encode(&publish(&file, *error));
            script.push(format!("sleep {pause}"));
            script.push(cat(format!("publish{i}"), publish));
        }
        // The copy runs alongside the script, not after it, so a test hears
        // what crew sent while the publishes are still to come. A background
        // job's stdin is /dev/null unless redirected, hence fd 3.
        let heard = dir.join("heard");
        let script = format!(
            "exec 3<&0; cat <&3 > '{}' & {}; wait",
            heard.display(),
            script.join("; ")
        );
        let server = Server {
            command: "sh".into(),
            args: vec!["-c".into(), script],
        };
        let host = LspHost::new(BTreeMap::from([("rust".to_string(), server)]));
        Self {
            host,
            dir,
            file,
            heard,
            hovers: 0,
        }
    }

    pub(super) fn uri(&self) -> String {
        crew_lsp::uri::from_path(&self.file)
    }

    pub(super) fn write(&self, text: &str) {
        std::fs::write(&self.file, text).unwrap();
    }

    pub(super) fn args(&self) -> String {
        json!({"file": self.file, "line": 1, "col": 1}).to_string()
    }

    /// Every whole frame the server has been sent so far.
    fn frames(&self) -> Vec<Value> {
        let bytes = std::fs::read(&self.heard).unwrap_or_default();
        let mut r = std::io::BufReader::new(bytes.as_slice());
        std::iter::from_fn(|| decode(&mut r).ok().flatten()).collect()
    }

    /// The `textDocument/did…` notifications, in order, once `done` holds
    /// of the frames heard (or five seconds have gone, for a red test).
    pub(super) fn dids_once(&self, done: impl Fn(&[Value]) -> bool) -> Vec<Value> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut frames = self.frames();
        while !done(&frames) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
            frames = self.frames();
        }
        let did = |m: &Value| {
            m["method"]
                .as_str()
                .is_some_and(|s| s.starts_with("textDocument/did"))
        };
        frames.into_iter().filter(did).collect()
    }

    /// Ask a hover, and return every `did…` the server heard before the
    /// hover itself reached it — which is everything the call sent first.
    pub(super) fn hover(&mut self) -> Vec<Value> {
        self.host.call("hover", &self.args()).unwrap();
        self.hovers += 1;
        let n = self.hovers;
        self.dids_once(|f| {
            f.iter()
                .filter(|m| m["method"] == "textDocument/hover")
                .count()
                >= n
        })
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn publish(file: &Path, error: Option<&str>) -> Value {
    let diagnostics: Vec<Value> = error
        .into_iter()
        .map(|m| {
            json!({"range": {"start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 2}}, "severity": 1, "message": m})
        })
        .collect();
    json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
        "params": {"uri": crew_lsp::uri::from_path(file), "diagnostics": diagnostics}})
}

pub(super) fn names(frames: &[Value]) -> Vec<&str> {
    frames
        .iter()
        .map(|m| m["method"].as_str().unwrap())
        .collect()
}
