pub mod active_window;
pub mod cursor;
pub mod monitor;

use serde::Serialize;

/// A rectangle in physical screen pixels.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
