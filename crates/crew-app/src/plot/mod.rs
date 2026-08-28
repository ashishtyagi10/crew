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
pub mod area;
pub mod canvas;
pub mod pie;

pub use canvas::Canvas;
