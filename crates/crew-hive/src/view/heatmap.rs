//! Heatmap layout: row-major grid of agent cells.
use crate::bus::AgentId;
use crate::telemetry::Fleet;
use crate::view::{state_color, Constellation, Rgb};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub agent: AgentId,
    pub row: usize,
    pub col: usize,
    pub color: Rgb,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Heatmap {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FleetView {
    Constellation(Constellation),
    Heatmap(Heatmap),
}

pub fn heatmap(fleet: &Fleet, cols: usize) -> Heatmap {
    let cols = cols.max(1);
    let agents: Vec<_> = fleet.agents().collect();
    let n = agents.len();
    let rows = if n == 0 { 0 } else { (n + cols - 1) / cols };
    let cells = agents
        .iter()
        .enumerate()
        .map(|(i, a)| Cell {
            agent: a.agent.clone(),
            row: i / cols,
            col: i % cols,
            color: state_color(a.state),
        })
        .collect();
    Heatmap { cols, rows, cells }
}
