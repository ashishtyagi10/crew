use crew_plugin::{plan, PluginCommand};
use std::io::{BufRead, Write};

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(cmd) = serde_json::from_str::<PluginCommand>(&line) else {
            continue;
        };
        for ev in plan(&cmd) {
            writeln!(out, "{}", serde_json::to_string(&ev)?)?;
        }
        out.flush()?;
    }
    Ok(())
}
