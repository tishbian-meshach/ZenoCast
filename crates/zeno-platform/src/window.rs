//! Windows desktop top-level window enumeration for capture targeting.

use std::mem::zeroed;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, IsIconic,
    IsWindowVisible,
};

/// Information about an active top-level desktop window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// Native window handle identifier (HWND as isize).
    pub hwnd: isize,
    /// Window title bar text.
    pub title: String,
    /// Window class name (e.g. "Chrome_WidgetWin_1", "Notepad").
    pub class_name: String,
    /// Pixel position and size (x, y, width, height).
    pub bounds: (i32, i32, u32, u32),
    /// Whether the window is currently minimized.
    pub is_minimized: bool,
}

/// Enumerates all currently visible, capturable top-level desktop application windows.
pub fn enumerate_windows() -> Vec<WindowInfo> {
    let mut windows_list = Vec::new();

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is a pointer to Vec<WindowInfo> passed into EnumWindows.
        let list = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };

        // SAFETY: Standard Win32 query on HWND.
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return BOOL(1);
        }

        // SAFETY: Query window title length.
        let text_len = unsafe { GetWindowTextLengthW(hwnd) };
        if text_len == 0 {
            return BOOL(1);
        }

        let mut title_buf = vec![0u16; (text_len + 1) as usize];
        // SAFETY: Buffer is pre-allocated with length text_len + 1.
        let copied = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
        let title = String::from_utf16_lossy(&title_buf[..copied as usize]);

        let mut class_buf = [0u16; 256];
        // SAFETY: Buffer is fixed stack array of 256 u16.
        let class_len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
        let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

        // Filter out common Windows internal shell containers
        if class_name == "Progman"
            || class_name == "Shell_TrayWnd"
            || class_name == "Windows.UI.Core.CoreWindow"
        {
            return BOOL(1);
        }

        // SAFETY: Query window client/frame rectangle.
        let mut rect: RECT = unsafe { zeroed() };
        let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
        let width = (rect.right - rect.left).unsigned_abs();
        let height = (rect.bottom - rect.top).unsigned_abs();

        if width < 8 || height < 8 {
            return BOOL(1);
        }

        // SAFETY: Query minimization state.
        let is_minimized = unsafe { IsIconic(hwnd) }.as_bool();

        list.push(WindowInfo {
            hwnd: hwnd.0 as isize,
            title,
            class_name,
            bounds: (rect.left, rect.top, width, height),
            is_minimized,
        });

        BOOL(1)
    }

    // SAFETY: Passing callback and pointer to our vector into EnumWindows.
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut windows_list as *mut _ as isize),
        );
    }

    windows_list
}
