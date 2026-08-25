use serde::Serialize;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, GetWindowRect,
};

use super::monitor;
use super::Rect;

#[derive(Clone, Debug, Serialize)]
pub struct ActiveAppInfo {
    pub process_name: String,
    pub app_name: String,
    pub window_title: String,
    pub pid: u32,
    pub bounds: Rect,
    pub monitor_device: String,
}

fn window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

fn process_image_name(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let result =
            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        result.ok()?;
        Some(String::from_utf16_lossy(&buf[..size as usize]))
    }
}

/// Friendly display names for common applications.
fn prettify(process_name: &str) -> String {
    let lower = process_name.to_lowercase();
    match lower.as_str() {
        "code" => "Visual Studio Code",
        "msedge" => "Microsoft Edge",
        "chrome" => "Google Chrome",
        "firefox" => "Firefox",
        "explorer" => "File Explorer",
        "discord" => "Discord",
        "spotify" => "Spotify",
        "wechat" | "weixin" => "WeChat",
        "notion" => "Notion",
        "slack" => "Slack",
        "telegram" => "Telegram",
        "winword" => "Microsoft Word",
        "excel" => "Microsoft Excel",
        "powerpnt" => "Microsoft PowerPoint",
        "outlook" => "Microsoft Outlook",
        "devenv" => "Visual Studio",
        "cmd" => "Command Prompt",
        "powershell" | "pwsh" => "PowerShell",
        "windowsterminal" | "wt" => "Windows Terminal",
        "premierepro" => "Adobe Premiere Pro",
        "photoshop" => "Adobe Photoshop",
        "figma" => "Figma",
        "obs64" => "OBS Studio",
        "steam" => "Steam",
        other => return capitalize(other),
    }
    .to_string()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn window_rect(hwnd: HWND) -> Option<Rect> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    Some(Rect {
        x: rect.left,
        y: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    })
}

/// Information about the current foreground application.
pub fn foreground_app_info() -> Option<ActiveAppInfo> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }

    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }

    let image = process_image_name(pid);
    let process_name = image
        .as_deref()
        .and_then(|path| {
            std::path::Path::new(path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let monitor = monitor::monitor_for_window(hwnd);
    let bounds = window_rect(hwnd)?;
    let monitor_device = monitor.map(|m| m.device).unwrap_or_default();

    Some(ActiveAppInfo {
        app_name: prettify(&process_name),
        process_name,
        window_title: window_title(hwnd),
        pid,
        bounds,
        monitor_device,
    })
}
