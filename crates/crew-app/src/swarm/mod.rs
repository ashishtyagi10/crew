//! Swarm integration: off-thread scheduler bridge + Fleet→CellViews renderer.
pub mod backend;
pub mod bridge;
pub mod compose;
pub mod plan;
pub mod rows;
#[cfg(test)]
mod tests;
pub mod timeline;
pub mod view;
