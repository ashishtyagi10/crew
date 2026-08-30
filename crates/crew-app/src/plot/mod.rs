//! `plot` — crew's chart toolkit: a sub-cell raster canvas and the widgets
//! drawn on it.
//!
//! Everything crew showed before this module was spelled in glyphs: gauges out
//! of `█`, sparklines out of the eighth-block ramp, tables out of box-drawing.
//! Glyph charts top out at eight levels and one sample per column, and they
//! cannot draw a curve, an arc or a slice at all. [`Canvas`] rasterizes real
//! shapes at sub-cell resolution and hands the frame a list of
//! [`Paint`](crew_render::Paint) rectangles, so a widget can describe a circle
//! and get a circle.
//!
//! Curves are described as [signed distances](sdf) rather than as
//! inside/outside tests: the canvas can sample either, but only a distance
//! anti-aliases a twenty-pixel arc without a staircase on it.
pub mod area;
pub mod canvas;
pub mod device;
pub mod dial;
pub mod gantt;
pub mod heatmap;
pub mod meter;
pub mod pie;
pub mod sdf;
pub mod treemap;

pub use canvas::Canvas;
