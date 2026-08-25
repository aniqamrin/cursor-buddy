use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HMONITOR, MONITORINFO, MONITORINFOEXW,
    MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::Foundation::HWND;

use super::Rect;

/// A physical monitor with its work area and effective DPI.
#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub device: String,
    pub work_area: Rect,
    pub dpi: u32,
}

impl MonitorInfo {
    /// Scale factor relative to a 96-dpi baseline (1.0 = 100%).
    pub fn scale(&self) -> f32 {
        self.dpi as f32 / 96.0
    }

    /// Convert a logical (96-dpi) size to this monitor's physical pixels.
    pub fn scale_logical_size(&self, width: i32, height: i32) -> (i32, i32) {
        (
            (width as f32 * self.scale()).round() as i32,
            (height as f32 * self.scale()).round() as i32,
        )
    }
}

fn query_monitor(hmon: HMONITOR) -> Option<MonitorInfo> {
    unsafe {
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        let _ = GetMonitorInfoW(hmon, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO);

        let mi = &info.monitorInfo;
        let wa = mi.rcWork;
        let work_area = Rect {
            x: wa.left,
            y: wa.top,
            width: wa.right - wa.left,
            height: wa.bottom - wa.top,
        };

        let device = String::from_utf16_lossy(
            &info.szDevice[..info.szDevice.iter().position(|&c| c == 0).unwrap_or(0)],
        );

        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        // Effective DPI; falls back silently to 96 on unsupported configs.
        let _ = GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);

        Some(MonitorInfo {
            device,
            work_area,
            dpi: dpi_x.max(dpi_y),
        })
    }
}

pub fn monitor_for_point(x: i32, y: i32) -> Option<MonitorInfo> {
    let pt = POINT { x, y };
    let hmon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    if hmon.is_invalid() {
        None
    } else {
        query_monitor(hmon)
    }
}

pub fn monitor_for_window(hwnd: HWND) -> Option<MonitorInfo> {
    let hmon = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if hmon.is_invalid() {
        None
    } else {
        query_monitor(hmon)
    }
}

/// Best-effort primary monitor lookup (used as a safe fallback).
pub fn primary_monitor() -> Option<MonitorInfo> {
    monitor_for_point(0, 0)
}
