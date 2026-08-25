use serde::Serialize;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// Physical screen coordinates of the mouse cursor.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

pub fn get_cursor_position() -> Result<CursorPosition, String> {
    let mut pt = POINT::default();
    unsafe { GetCursorPos(&mut pt) }.map_err(|e| format!("GetCursorPos failed: {e}"))?;
    Ok(CursorPosition { x: pt.x, y: pt.y })
}
