//! Swarm integration: off-thread scheduler bridge + Fleet→CellViews renderer.
pub mod backend;
pub mod bridge;
pub mod plan;
#[cfg(test)]
mod tests;
pub mod timeline;
pub mod view;
