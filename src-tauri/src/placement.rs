use crate::windows::Rect;

/// Margin kept from screen edges (physical px).
pub const EDGE_MARGIN: i32 = 12;
/// Gap between cursor and bubble (physical px).
pub const CURSOR_GAP: i32 = 10;

/// Compute the top-left position for a popup near the cursor.
///
/// Prefers bottom-right of the cursor; flips left/up when it would overflow
/// the monitor work area, then clamps so the popup is never off-screen.
/// All values are physical pixels.
pub fn place_bubble(
    cursor: (i32, i32),
    size: (i32, i32),
    work_area: Rect,
    margin: i32,
    gap: i32,
) -> (i32, i32) {
    let (w, h) = size;
    let right = work_area.x + work_area.width - margin;
    let bottom = work_area.y + work_area.height - margin;
    let min_x = work_area.x + margin;
    let min_y = work_area.y + margin;

    let mut x = cursor.0 + gap;
    let mut y = cursor.1 + gap;

    // Flip horizontally when overflowing the right edge.
    if x + w > right {
        x = cursor.0 - gap - w;
    }
    // Flip vertically when overflowing the bottom edge.
    if y + h > bottom {
        y = cursor.1 - gap - h;
    }

    // Final clamp guarantees on-screen placement even for huge popups.
    x = x.clamp(min_x, (right - w).max(min_x));
    y = y.clamp(min_y, (bottom - h).max(min_y));

    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, width: w, height: h }
    }

    const BUBBLE: (i32, i32) = (400, 340);

    #[test]
    fn places_bottom_right_by_default() {
        let pos = place_bubble((100, 100), BUBBLE, area(0, 0, 1920, 1040), 12, 10);
        assert_eq!(pos, (110, 110));
    }

    #[test]
    fn flips_left_near_right_edge() {
        let pos = place_bubble((1900, 300), BUBBLE, area(0, 0, 1920, 1040), 12, 10);
        assert!(pos.0 + BUBBLE.0 <= 1920 - 12, "must stay inside right margin");
        assert!(pos.0 < 1900, "bubble flips to the left of the cursor");
    }

    #[test]
    fn flips_up_near_bottom_edge() {
        let pos = place_bubble((500, 1020), BUBBLE, area(0, 0, 1920, 1040), 12, 10);
        assert!(pos.1 + BUBBLE.1 <= 1040 - 12, "must stay inside bottom margin");
        assert!(pos.1 < 1020, "bubble flips above the cursor");
    }

    #[test]
    fn flips_both_near_corner() {
        let pos = place_bubble((1910, 1030), BUBBLE, area(0, 0, 1920, 1040), 12, 10);
        assert!(pos.0 + BUBBLE.0 <= 1920 - 12, "inside right margin");
        assert!(pos.1 + BUBBLE.1 <= 1040 - 12, "inside bottom margin");
    }

    #[test]
    fn clamps_inside_work_area_when_popup_huge() {
        let pos = place_bubble((960, 520), (2500, 1400), area(0, 0, 1920, 1040), 12, 10);
        assert!(pos.0 >= 12 && pos.1 >= 12);
        assert!(pos.0 + 2500 <= 1920 - 12 || pos.0 == 12);
    }

    #[test]
    fn respects_work_area_offset_secondary_monitor() {
        // Secondary monitor to the right: x starts at 1920.
        let wa = area(1920, 0, 2560, 1392);
        let pos = place_bubble((4400, 700), BUBBLE, wa, 12, 10);
        assert!(pos.0 >= 1920 + 12);
        assert!(pos.0 + BUBBLE.0 <= 1920 + 2560 - 12);
    }
}
