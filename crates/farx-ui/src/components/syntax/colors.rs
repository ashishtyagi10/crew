//! Color palette for syntax highlighting — rich, distinct colors.

use ratatui::style::Color;

// Background colors available for syntax-highlighted rendering
pub(super) const _BG: Color = Color::Rgb(22, 22, 26);
pub(super) const _BG_CURSOR: Color = Color::Indexed(236);

pub(super) const C_KEYWORD: Color = Color::Rgb(220, 170, 60); // amber — keywords (if, for, fn)
pub(super) const C_CONTROL: Color = Color::Rgb(230, 120, 100); // coral — control flow (return, break)
pub(super) const C_STRING: Color = Color::Rgb(120, 195, 90); // green — strings
pub(super) const C_COMMENT: Color = Color::Rgb(95, 95, 120); // muted gray-blue — comments
pub(super) const C_NUMBER: Color = Color::Rgb(235, 145, 70); // orange — numbers
pub(super) const C_TYPE: Color = Color::Rgb(90, 185, 165); // teal — types/classes
pub(super) const C_BUILTIN: Color = Color::Rgb(130, 170, 220); // soft blue — builtin types/fns
pub(super) const C_FUNC: Color = Color::Rgb(200, 180, 130); // warm sand — function calls
pub(super) const C_OPERATOR: Color = Color::Rgb(195, 125, 175); // pink — operators
pub(super) const C_MACRO: Color = Color::Rgb(180, 150, 220); // lavender — macros/attributes/decorators
pub(super) const C_LIFETIME: Color = Color::Rgb(220, 140, 160); // rose — lifetimes ('a)
pub(super) const C_PLAIN: Color = Color::Rgb(192, 188, 180); // warm gray — default text
pub(super) const C_PUNCT: Color = Color::Rgb(120, 118, 112); // dim — punctuation/brackets
pub(super) const C_SPECIAL: Color = Color::Rgb(160, 200, 230); // light blue — self/this/super
