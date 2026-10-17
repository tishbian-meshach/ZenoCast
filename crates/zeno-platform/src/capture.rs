//! Windows screen and window frame capture routines with cursor overlay and DIB extraction.

use std::mem::zeroed;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    GetWindowDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DrawIconEx, GetCursorInfo, GetIconInfo, GetWindowRect, CURSORINFO, CURSOR_SHOWING, DI_NORMAL,
    HICON,
};

/// Captures a desktop top-level window image into preallocated RGBA8 pixel memory.
pub fn capture_window_raw(
    hwnd_val: isize,
    capture_cursor: bool,
    out_width: &mut u32,
    out_height: &mut u32,
    out_rgba: &mut Vec<u8>,
) -> Result<(), String> {
    let hwnd = HWND(hwnd_val as *mut _);

    // SAFETY: Query window dimensions.
    let mut rect: RECT = unsafe { zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return Err("Failed to query window rectangle".to_string());
    }

    let width = (rect.right - rect.left).unsigned_abs().max(1);
    let height = (rect.bottom - rect.top).unsigned_abs().max(1);
    *out_width = width;
    *out_height = height;

    // SAFETY: Get window DC and create compatible memory DC.
    let hdc_src = unsafe { GetWindowDC(hwnd) };
    if hdc_src.is_invalid() {
        return Err("Failed to obtain window device context".to_string());
    }

    let hdc_mem = unsafe { CreateCompatibleDC(hdc_src) };
    if hdc_mem.is_invalid() {
        unsafe { ReleaseDC(hwnd, hdc_src) };
        return Err("Failed to create compatible memory DC".to_string());
    }

    let hbm = unsafe { CreateCompatibleBitmap(hdc_src, width as i32, height as i32) };
    if hbm.is_invalid() {
        unsafe {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_src);
        }
        return Err("Failed to allocate compatible bitmap".to_string());
    }

    // SAFETY: Select bitmap into memory DC, copy pixels, and restore old object.
    let old_bmp = unsafe { SelectObject(hdc_mem, hbm) };
    let blt_res = unsafe {
        BitBlt(
            hdc_mem,
            0,
            0,
            width as i32,
            height as i32,
            hdc_src,
            0,
            0,
            SRCCOPY,
        )
    };

    if capture_cursor {
        // SAFETY: Query cursor info and overlay onto memory DC if inside window bounds.
        let mut cursor_info: CURSORINFO = unsafe { zeroed() };
        cursor_info.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        if unsafe { GetCursorInfo(&mut cursor_info) }.is_ok() && cursor_info.flags == CURSOR_SHOWING
        {
            let cursor_x = cursor_info.ptScreenPos.x - rect.left;
            let cursor_y = cursor_info.ptScreenPos.y - rect.top;
            if cursor_x >= 0 && cursor_x < width as i32 && cursor_y >= 0 && cursor_y < height as i32
            {
                let _ = unsafe {
                    DrawIconEx(
                        hdc_mem,
                        cursor_x,
                        cursor_y,
                        HICON(cursor_info.hCursor.0),
                        0,
                        0,
                        0,
                        None,
                        DI_NORMAL,
                    )
                };
            }
        }
    }

    if blt_res.is_err() {
        unsafe {
            SelectObject(hdc_mem, old_bmp);
            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_src);
        }
        return Err("BitBlt window transfer failed".to_string());
    }

    // Read pixel buffer via GetDIBits
    let byte_count = (width * height * 4) as usize;
    if out_rgba.len() < byte_count {
        out_rgba.resize(byte_count, 0);
    }

    let mut bmi: BITMAPINFO = unsafe { zeroed() };
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32), // Top-down DIB
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };

    // SAFETY: Read DIB scanlines into out_rgba memory buffer.
    let lines = unsafe {
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            height,
            Some(out_rgba.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    // Convert BGRA to RGBA in-place
    if lines > 0 {
        for chunk in out_rgba[..byte_count].chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
            chunk[3] = 255; // Opaque alpha
        }
    }

    // Clean up GDI handles
    unsafe {
        SelectObject(hdc_mem, old_bmp);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_src);
    }

    if lines == 0 {
        Err("GetDIBits failed to extract window scanlines".to_string())
    } else {
        Ok(())
    }
}

/// Captures a physical monitor screen image into preallocated RGBA8 pixel memory.
pub fn capture_display_raw(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    capture_cursor: bool,
    out_rgba: &mut Vec<u8>,
) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("Display bounds have zero dimensions".to_string());
    }

    // SAFETY: Retrieve root desktop DC.
    let hdc_screen = unsafe { GetDC(HWND(std::ptr::null_mut())) };
    if hdc_screen.is_invalid() {
        return Err("Failed to obtain desktop device context".to_string());
    }

    let hdc_mem = unsafe { CreateCompatibleDC(hdc_screen) };
    if hdc_mem.is_invalid() {
        unsafe { ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen) };
        return Err("Failed to create compatible memory DC".to_string());
    }

    let hbm = unsafe { CreateCompatibleBitmap(hdc_screen, width as i32, height as i32) };
    if hbm.is_invalid() {
        unsafe {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
        }
        return Err("Failed to allocate compatible bitmap".to_string());
    }

    let old_bmp = unsafe { SelectObject(hdc_mem, hbm) };
    let blt_res = unsafe {
        BitBlt(
            hdc_mem,
            0,
            0,
            width as i32,
            height as i32,
            hdc_screen,
            x,
            y,
            SRCCOPY,
        )
    };

    if capture_cursor {
        // SAFETY: Query cursor info and overlay onto memory DC if inside monitor bounds.
        let mut cursor_info: CURSORINFO = unsafe { zeroed() };
        cursor_info.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        if unsafe { GetCursorInfo(&mut cursor_info) }.is_ok() && cursor_info.flags == CURSOR_SHOWING
        {
            let cursor_x = cursor_info.ptScreenPos.x - x;
            let cursor_y = cursor_info.ptScreenPos.y - y;
            if cursor_x >= 0 && cursor_x < width as i32 && cursor_y >= 0 && cursor_y < height as i32
            {
                let mut icon_info = unsafe { zeroed() };
                let _ = unsafe { GetIconInfo(HICON(cursor_info.hCursor.0), &mut icon_info) };
                if !icon_info.hbmColor.is_invalid() {
                    unsafe {
                        let _ = DeleteObject(icon_info.hbmColor);
                    };
                }
                if !icon_info.hbmMask.is_invalid() {
                    unsafe {
                        let _ = DeleteObject(icon_info.hbmMask);
                    };
                }

                let _ = unsafe {
                    DrawIconEx(
                        hdc_mem,
                        cursor_x - icon_info.xHotspot as i32,
                        cursor_y - icon_info.yHotspot as i32,
                        HICON(cursor_info.hCursor.0),
                        0,
                        0,
                        0,
                        None,
                        DI_NORMAL,
                    )
                };
            }
        }
    }

    if blt_res.is_err() {
        unsafe {
            SelectObject(hdc_mem, old_bmp);
            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
        }
        return Err("BitBlt screen transfer failed".to_string());
    }

    let byte_count = (width * height * 4) as usize;
    if out_rgba.len() < byte_count {
        out_rgba.resize(byte_count, 0);
    }

    let mut bmi: BITMAPINFO = unsafe { zeroed() };
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32), // Top-down
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };

    let lines = unsafe {
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            height,
            Some(out_rgba.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    if lines > 0 {
        for chunk in out_rgba[..byte_count].chunks_exact_mut(4) {
            chunk.swap(0, 2); // BGRA to RGBA
            chunk[3] = 255;
        }
    }

    unsafe {
        SelectObject(hdc_mem, old_bmp);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
    }

    if lines == 0 {
        Err("GetDIBits failed to extract screen scanlines".to_string())
    } else {
        Ok(())
    }
}
