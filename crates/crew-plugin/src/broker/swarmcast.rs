//! A run's cast, said and stored: the plan line, the specialist record, and
//! the roster re-emit. Split from `swarm.rs` so the verification pass could
//! land there without the file growing past its debt — nothing here changed
//! in the move. A child of `swarm`, so `broker` items are reached through
//! `crate::broker::`.
use crew_hive::TaskSpec;

use super::SWARM_LEAD;
use crate::broker::relay::msg;
use crate::protocol::PluginEvent;

/// Say the plan, persist its cast, then re-emit the roster: `Roster` is
/// otherwise only sent from `hello()`, so without this the app never learns
/// about a specialist invented mid-session and the new names never appear.
/// `model` is the slug serving the run's API agents (empty when unknown).
pub(super) fn announce(
    tasks: &[TaskSpec],
    model: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    emit(msg(
        SWARM_LEAD,
        format!(
            "planned {} task(s): {}",
            tasks.len(),
            tasks
                .iter()
                .map(|t| t.title.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))?;
    // First-wins on a duplicate name: one name is one specialist.
    let mut seen: Vec<(String, String)> = Vec::new();
    for t in tasks {
        if !seen.iter().any(|(n, _)| n == &t.specialty) {
            seen.push((t.specialty.clone(), t.expertise.clone()));
        }
    }
    crate::broker::specialists::record(&seen);
    // The roster leads with the run's own cast, stamped with the model
    // serving it, built from memory: `record` above is best-effort (a broker
    // launched from Finder/Dock runs at `/`, where `.crew/` is unwritable),
    // and a disk re-read would come back empty exactly then — taking the
    // footer's model segment with it. Discovery still appends everyone the
    // cast doesn't name (CLI agents, manifest plugins, stored specialists).
    let mut agents: Vec<crate::AgentInfo> = seen
        .iter()
        .map(|(name, role)| crate::AgentInfo {
            name: name.clone(),
            role: role.clone(),
            model: model.to_string(),
        })
        .collect();
    for info in crate::broker::Registry::discover().infos() {
        if !agents.iter().any(|a| a.name == info.name) {
            agents.push(info);
        }
    }
    emit(crate::broker::rosterev::roster(agents))
}
