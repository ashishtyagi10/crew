//! The frame path's two invariants, read off the source.
//!
//! Neither can be exercised from a test: an occluded surface needs a display
//! the machine has turned off, and the cost of getting it wrong is invisible
//! until a night has passed. So they are asserted as SHAPE — the order of two
//! calls, and the form of every early return — which is exactly what a later
//! edit would break without noticing.
const SRC: &str = include_str!("frame.rs");

/// The body of `render`, from its opening brace to the `submit` that ends it.
fn render_body() -> &'static str {
    let start = SRC
        .find("panes: &[PaneScene],\n) {")
        .expect("render's signature moved");
    let end = SRC.find("gpu.queue().submit").expect("the submit moved");
    assert!(start < end, "the submit is above render's body");
    &SRC[start..end]
}

#[test]
fn the_surface_is_acquired_before_anything_is_uploaded() {
    let body = render_body();
    let acquire = body
        .find("get_current_texture")
        .expect("no surface acquisition");
    let upload = body.find("cell_grid.set_scene").expect("no scene upload");
    assert!(
        acquire < upload,
        "the scene is uploaded before the surface is asked for it: a frame the \
         surface then refuses has allocated buffers that nothing will free"
    );
    assert!(
        body.find("cell_grid.prepare").is_some_and(|p| acquire < p),
        "glyphon's prepare runs before the surface is acquired"
    );
}

#[test]
fn every_early_return_maintains_the_device() {
    let body = render_body();
    let bare: Vec<&str> = body
        .match_indices("return")
        .map(|(i, _)| body[i..].lines().next().unwrap_or(""))
        .filter(|line| !line.contains("drain(gpu)"))
        .collect();
    assert!(
        bare.is_empty(),
        "a frame that returns without submitting must drain the device, or \
         the buffers it dropped are never freed:\n  {}",
        bare.join("\n  ")
    );
}

#[test]
fn the_drain_never_blocks_on_the_gpu() {
    // `Wait` here would park the winit thread — which is every pane's thread —
    // on a GPU that is, by construction, not presenting anything.
    let drain = SRC.split("fn drain(").nth(1).expect("drain is gone");
    assert!(drain.contains("PollType::Poll"), "{drain}");
    assert!(
        !drain.contains("PollType::Wait"),
        "the drain waits: {drain}"
    );
}
