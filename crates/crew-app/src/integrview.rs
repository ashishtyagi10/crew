//! `/integrations`: what crew can reach, and whether each will work.
//!
//! `/tools` says what RAN; nothing in the app said what is AVAILABLE. The
//! only answer to "what can crew reach, and will it work" was one line per
//! integration in `/doctor`, buried among fifteen other probes — and a
//! manifest whose token is missing works perfectly until its first call.
//! This is the health view the JARVIS goal asks for: every manifest, the
//! credential it names and whether that is set, and every tool with its
//! tier, so the irreversible ones are known before the gate asks.
//!
//! Read fresh each time, like `/watching`: the manifests are re-read per hop
//! anyway, so what this shows is what the next call will see.
use crew_plugin::integration::{Auth, Integration};

#[cfg(test)]
#[path = "integrview_tests.rs"]
pub(crate) mod tests;

/// Columns a row may take — the tile width `/tools` fits to.
#[cfg(test)]
const ROW_W: usize = crate::toolsrow::ROW_W;
/// The narrowest the tool column goes, so short names still line up.
const TOOL_W_MIN: usize = 22;

/// The widest it goes: what a tile holds beside the widest tier —
/// two of indent, the name, a space and `irreversible`.
const TOOL_W_MAX: usize = crate::toolsrow::ROW_W - 2 - 1 - 12;

/// Columns the tool column takes: the longest name across every
/// integration, so nothing a tile can hold whole is cut in the middle.
/// `subscribe_to_weather_alerts` was `subscribe_t…her_alerts` at a fixed
/// 22, in a pane with forty columns to spare.
fn tool_col(ints: &[Integration]) -> usize {
    ints.iter()
        .flat_map(|i| &i.tools)
        .map(|t| crate::chatwidth::str_w(&t.name))
        .max()
        .unwrap_or(0)
        .clamp(TOOL_W_MIN, TOOL_W_MAX)
}

/// The credential an integration names, and whether it is set — `set` is
/// asked rather than the environment, so a test can say either.
fn credential(i: &Integration, set: &dyn Fn(&str) -> bool) -> String {
    let env = match &i.auth {
        Auth::Bearer { env } | Auth::Header { env, .. } | Auth::Query { env, .. } => env,
        Auth::None => return "no credential needed".into(),
    };
    if set(env) {
        format!("{env} is set")
    } else {
        format!("{env} is NOT set \u{2014} calls will refuse")
    }
}

/// The listing for `ints`, as viewer text.
pub(crate) fn listing(ints: &[Integration], set: &dyn Fn(&str) -> bool) -> String {
    let mut out = String::from("# integrations \u{b7} what crew can reach\n");
    if ints.is_empty() {
        // One paragraph, for the viewer to wrap at its own width (see
        // `watchview` for why).
        out.push_str(
            "None. Drop a manifest in ~/.config/crew/integrations/ (or a project's \
             .crew/integrations/) and /reload; examples/integrations/weather.json is one.\n",
        );
        return out;
    }
    let tools: usize = ints.iter().map(|i| i.tools.len()).sum();
    let w = tool_col(ints);
    out.push_str(&format!(
        "{} integration(s) \u{b7} {tools} tool(s) \u{b7} re-read per call\n\n",
        ints.len()
    ));
    for i in ints {
        out.push_str(&i.name);
        out.push('\n');
        // The credential, then the description: prose, one line each,
        // under the name they belong to. NOT wrapped here: the viewer's
        // plain rung wraps prose on words with a hanging indent at the
        // pane's real width, and a line pre-wrapped to the tile stayed
        // forty-seven columns wide in a pane of a hundred and twenty.
        out.push_str(&format!("  {}\n", credential(i, set)));
        if !i.description.trim().is_empty() {
            out.push_str(&format!("  {}\n", i.description.trim()));
        }
        for t in &i.tools {
            out.push_str(&format!(
                "  {:<w$} {}\n",
                crate::toolsrow::fit(&t.name, w),
                i.tier_of(&t.name).label()
            ));
        }
        out.push('\n');
    }
    out
}

impl crate::app::CrewApp {
    /// `/integrations` — the manifests crew has, in the viewer.
    pub(crate) fn open_integrations(&mut self) {
        let text = listing(&crew_plugin::integration::load(), &|env| {
            std::env::var(env).is_ok_and(|v| !v.trim().is_empty())
        });
        let path = crate::lastout::temp_path(usize::MAX, "integrations");
        if let Err(e) = std::fs::write(&path, text) {
            self.set_status(format!("integrations: cannot write: {e}"));
            return;
        }
        let before = self.panes.len();
        self.open_view(&path.to_string_lossy());
        self.name_last_view("integrations");
        self.mark_last_view_ephemeral(before);
    }
}
