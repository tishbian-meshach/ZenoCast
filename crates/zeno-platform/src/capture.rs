//! Windows screen and window frame capture routines with cursor overlay and DIB extraction.

use std::mem::zeroed;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::Interface;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::{IDirect3DDevice, IDirect3DSurface};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{BOOL, HWND, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    GetWindowDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    HDC, SRCCOPY,
};
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    DrawIconEx, GetCursorInfo, GetIconInfo, GetWindowRect, IsIconic, CURSORINFO, CURSOR_SHOWING,
    DI_NORMAL, HICON,
};

#[link(name = "user32")]
extern "system" {
    fn PrintWindow(hwnd: HWND, hdc_blt: HDC, n_flags: u32) -> BOOL;
}

/// A captured video frame with dimensions, RGBA8 pixel memory, and timestamp.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub timestamp_seconds: f64,
}

/// Windows Graphics Capture (WGC) hardware-accelerated session for a specific HWND.
pub struct WgcCaptureSession {
    hwnd: isize,
    session: GraphicsCaptureSession,
    frame_pool: Direct3D11CaptureFramePool,
    latest_frame: Arc<parking_lot::Mutex<Option<CapturedFrame>>>,
    active: Arc<AtomicBool>,
}

impl WgcCaptureSession {
    /// Creates and starts an asynchronous Windows Graphics Capture session for the target HWND.
    pub fn new(hwnd: isize, capture_cursor: bool) -> Result<Self, String> {
        let hwnd_val = HWND(hwnd as *mut _);

        // 1. Verify window exists and query rectangle
        let mut rect: RECT = unsafe { zeroed() };
        if unsafe { GetWindowRect(hwnd_val, &mut rect) }.is_err() {
            return Err("Invalid window handle".to_string());
        }
        let win_width = (rect.right - rect.left).unsigned_abs();
        let win_height = (rect.bottom - rect.top).unsigned_abs();

        if win_width == 0 || win_height == 0 {
            return Err("Window has zero dimensions".to_string());
        }

        // 2. Create GraphicsCaptureItem for HWND using IGraphicsCaptureItemInterop
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
            .map_err(|e| format!("Failed to get IGraphicsCaptureItemInterop: {}", e))?;

        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd_val) }
            .map_err(|e| format!("GraphicsCaptureItem::CreateForWindow failed: {}", e))?;

        let item_size = item
            .Size()
            .map_err(|e| format!("Failed to get item size: {}", e))?;

        tracing::info!(
            "[WGC] Starting window capture for HWND 0x{:x}, item size: {}x{}, window rect: {}x{}",
            hwnd,
            item_size.Width,
            item_size.Height,
            win_width,
            win_height
        );

        if item_size.Width <= 0 || item_size.Height <= 0 {
            return Err("Captured item has non-positive size".to_string());
        }

        // 3. Create Direct3D11 Device
        let mut d3d_device: Option<ID3D11Device> = None;
        let mut d3d_context: Option<ID3D11DeviceContext> = None;
        let feature_levels = [D3D_FEATURE_LEVEL_11_0];
        let mut feature_level = D3D_FEATURE_LEVEL_11_0;

        let hr = unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                Some(&mut feature_level),
                Some(&mut d3d_context),
            )
        };
        if let Err(e) = hr {
            return Err(format!("D3D11CreateDevice failed: {}", e));
        }
        let d3d_device = d3d_device.ok_or_else(|| "Failed to obtain D3D11 device".to_string())?;
        let d3d_context =
            d3d_context.ok_or_else(|| "Failed to obtain D3D11 context".to_string())?;

        // 4. Create WinRT IDirect3DDevice from DXGI device
        let dxgi_device: IDXGIDevice = d3d_device
            .cast()
            .map_err(|e| format!("Failed to cast D3D11 device to IDXGIDevice: {}", e))?;
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }
            .map_err(|e| format!("CreateDirect3D11DeviceFromDXGIDevice failed: {}", e))?;
        let winrt_device: IDirect3DDevice = inspectable
            .cast()
            .map_err(|e| format!("Failed to cast to IDirect3DDevice: {}", e))?;

        // 5. Create FreeThreaded FramePool (buffer_count = 2 for double buffering)
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            item_size,
        )
        .map_err(|e| {
            format!(
                "Direct3D11CaptureFramePool::CreateFreeThreaded failed: {}",
                e
            )
        })?;

        // 6. Create CaptureSession
        let session = frame_pool
            .CreateCaptureSession(&item)
            .map_err(|e| format!("CreateCaptureSession failed: {}", e))?;

        // Configure cursor and border if supported
        let _ = session.SetIsCursorCaptureEnabled(capture_cursor);
        let _ = session.SetIsBorderRequired(false);

        let latest_frame = Arc::new(parking_lot::Mutex::new(None::<CapturedFrame>));
        let active = Arc::new(AtomicBool::new(true));

        // 7. Subscribe to FrameArrived on background thread
        let context_mutex = Arc::new(parking_lot::Mutex::new(d3d_context));
        let staging_tex = Arc::new(parking_lot::Mutex::new(None::<ID3D11Texture2D>));

        {
            let d3d_device = d3d_device.clone();
            let context_mutex = Arc::clone(&context_mutex);
            let staging_tex = Arc::clone(&staging_tex);
            let latest_frame = Arc::clone(&latest_frame);
            let active = Arc::clone(&active);

            frame_pool
                .FrameArrived(&TypedEventHandler::new(
                    move |pool_opt: &Option<Direct3D11CaptureFramePool>, _args| {
                        if !active.load(Ordering::Relaxed) {
                            return Ok(());
                        }
                        let Some(pool) = pool_opt else {
                            return Ok(());
                        };
                        let frame = match pool.TryGetNextFrame() {
                            Ok(f) => f,
                            Err(_) => return Ok(()),
                        };

                        let surface: IDirect3DSurface = frame.Surface()?;
                        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
                        let source_texture: ID3D11Texture2D = unsafe { access.GetInterface()? };

                        let mut desc = D3D11_TEXTURE2D_DESC::default();
                        unsafe { source_texture.GetDesc(&mut desc) };

                        let width = desc.Width;
                        let height = desc.Height;
                        if width == 0 || height == 0 {
                            let _ = frame.Close();
                            return Ok(());
                        }

                        let ctx = context_mutex.lock();
                        let mut staging_lock = staging_tex.lock();

                        let need_new = match staging_lock.as_ref() {
                            Some(tex) => {
                                let mut s_desc = D3D11_TEXTURE2D_DESC::default();
                                unsafe { tex.GetDesc(&mut s_desc) };
                                s_desc.Width != width || s_desc.Height != height
                            }
                            None => true,
                        };

                        if need_new {
                            let staging_desc = D3D11_TEXTURE2D_DESC {
                                Width: width,
                                Height: height,
                                MipLevels: 1,
                                ArraySize: 1,
                                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                                SampleDesc: DXGI_SAMPLE_DESC {
                                    Count: 1,
                                    Quality: 0,
                                },
                                Usage: D3D11_USAGE_STAGING,
                                BindFlags: 0,
                                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                                MiscFlags: 0,
                            };
                            let mut new_staging: Option<ID3D11Texture2D> = None;
                            let res = unsafe {
                                d3d_device.CreateTexture2D(
                                    &staging_desc,
                                    None,
                                    Some(&mut new_staging),
                                )
                            };
                            if res.is_ok() {
                                *staging_lock = new_staging;
                            }
                        }

                        if let Some(ref staging) = *staging_lock {
                            unsafe {
                                ctx.CopyResource(staging, &source_texture);
                                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                                if ctx
                                    .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                                    .is_ok()
                                {
                                    let row_pitch = mapped.RowPitch as usize;
                                    let pixel_count = (width * height) as usize;
                                    let mut rgba = vec![0u8; pixel_count * 4];

                                    let src_data = mapped.pData as *const u8;
                                    for y in 0..height as usize {
                                        let src_row = std::slice::from_raw_parts(
                                            src_data.add(y * row_pitch),
                                            width as usize * 4,
                                        );
                                        let dst_row = &mut rgba[y * (width as usize * 4)
                                            ..(y + 1) * (width as usize * 4)];

                                        for (s, d) in
                                            src_row.chunks_exact(4).zip(dst_row.chunks_exact_mut(4))
                                        {
                                            d[0] = s[2]; // R
                                            d[1] = s[1]; // G
                                            d[2] = s[0]; // B
                                            d[3] = 255; // A
                                        }
                                    }

                                    ctx.Unmap(staging, 0);

                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_secs_f64())
                                        .unwrap_or(0.0);

                                    *latest_frame.lock() = Some(CapturedFrame {
                                        width,
                                        height,
                                        rgba,
                                        timestamp_seconds: now,
                                    });
                                }
                            }
                        }

                        drop(staging_lock);
                        drop(ctx);
                        let _ = frame.Close();
                        Ok(())
                    },
                ))
                .map_err(|e| format!("FrameArrived registration failed: {}", e))?;
        }

        session
            .StartCapture()
            .map_err(|e| format!("StartCapture failed: {}", e))?;

        Ok(Self {
            hwnd,
            session,
            frame_pool,
            latest_frame,
            active,
        })
    }

    /// Queries the latest captured frame from the background pool without blocking.
    pub fn get_latest_frame(&self) -> Option<CapturedFrame> {
        self.latest_frame.lock().clone()
    }

    /// Returns the target HWND handle.
    pub fn hwnd(&self) -> isize {
        self.hwnd
    }

    /// Stops and closes the capture session.
    pub fn stop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
        let _ = self.session.Close();
        let _ = self.frame_pool.Close();
    }
}

impl Drop for WgcCaptureSession {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Captures a desktop top-level window image into preallocated RGBA8 pixel memory.
pub fn capture_window_raw(
    hwnd_val: isize,
    capture_cursor: bool,
    out_width: &mut u32,
    out_height: &mut u32,
    out_rgba: &mut Vec<u8>,
) -> Result<(), String> {
    let hwnd = HWND(hwnd_val as *mut _);

    // SAFETY: Check minimization state.
    if unsafe { IsIconic(hwnd) }.as_bool() {
        return Err("Target window is minimized".to_string());
    }

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

    // Content transfer: First attempt PrintWindow with PW_RENDERFULLCONTENT (0x2) for hardware-accelerated DWM apps
    let mut blt_ok = unsafe { PrintWindow(hwnd, hdc_mem, 2) }.as_bool();
    if !blt_ok {
        // Fallback 1: PrintWindow standard (0x0)
        blt_ok = unsafe { PrintWindow(hwnd, hdc_mem, 0) }.as_bool();
    }
    if !blt_ok {
        // Fallback 2: BitBlt transfer
        blt_ok = unsafe {
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
        }
        .is_ok();
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

    // Inspect content: ensure the captured frame is not completely black/empty.
    // If all captured bytes are 0, the window content capture failed/no-oped.
    let has_content = lines > 0 && out_rgba[..byte_count.min(4096)].iter().any(|&b| b != 0);

    // Overlay cursor ONLY if window content was successfully retrieved.
    // Prevents the bug where a black frame with only a cursor was emitted.
    if has_content && capture_cursor {
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
                // Re-read scanlines after cursor overlay
                let _ = unsafe {
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
            }
        }
    }

    // Convert BGRA to RGBA in-place
    if lines > 0 {
        for chunk in out_rgba[..byte_count].chunks_exact_mut(4) {
            chunk.swap(0, 2);
            chunk[3] = 255;
        }
    }

    // Clean up GDI handles
    unsafe {
        SelectObject(hdc_mem, old_bmp);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_src);
    }

    if lines == 0 || !blt_ok {
        Err("Window content capture failed".to_string())
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
