//! Windows monitor and display enumeration.

use std::mem::zeroed;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};

/// Information about a detected physical monitor/display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayInfo {
    /// Native monitor handle identifier.
    pub hmonitor: isize,
    /// Friendly system device name (e.g. "\\\\.\\DISPLAY1").
    pub name: String,
    /// Monitor pixel bounds (x, y, width, height).
    pub bounds: (i32, i32, u32, u32),
    /// Whether this is the primary system monitor.
    pub is_primary: bool,
}

/// Enumerates all currently connected and active physical monitors.
pub fn enumerate_displays() -> Vec<DisplayInfo> {
    let mut displays = Vec::new();

    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        // SAFETY: lparam is a pointer to Vec<DisplayInfo> passed into EnumDisplayMonitors.
        let displays = unsafe { &mut *(lparam.0 as *mut Vec<DisplayInfo>) };

        // SAFETY: Initializing Windows API MONITORINFOEXW struct with its size field set.
        let mut info: MONITORINFOEXW = unsafe { zeroed() };
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

        // SAFETY: hmonitor is a valid handle passed by the OS into the monitor enumeration callback.
        if unsafe { GetMonitorInfoW(hmonitor, &mut info.monitorInfo as *mut _ as *mut _) }.as_bool()
        {
            let name_len = info
                .szDevice
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(info.szDevice.len());
            let name = String::from_utf16_lossy(&info.szDevice[..name_len]);

            let rect = info.monitorInfo.rcMonitor;
            let width = (rect.right - rect.left).unsigned_abs();
            let height = (rect.bottom - rect.top).unsigned_abs();
            let is_primary = (info.monitorInfo.dwFlags & 1) != 0;

            displays.push(DisplayInfo {
                hmonitor: hmonitor.0 as isize,
                name,
                bounds: (rect.left, rect.top, width, height),
                is_primary,
            });
        }

        BOOL(1)
    }

    // SAFETY: Passing a valid callback function and a raw pointer to our local Vec<DisplayInfo>.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut displays as *mut _ as isize),
        );
    }

    displays
}
