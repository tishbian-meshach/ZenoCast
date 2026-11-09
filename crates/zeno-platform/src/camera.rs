//! Windows Media Foundation camera capture, device enumeration, and hardware color conversion.

use crate::capture::CapturedFrame;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::Win32::Media::MediaFoundation::{
    IMFActivate, IMFAttributes, IMFMediaSource, IMFSample, IMFSourceReader, MFCreateAttributes,
    MFCreateMediaType, MFCreateSourceReaderFromMediaSource, MFEnumDeviceSources, MFMediaType_Video,
    MFShutdown, MFStartup, MFVideoFormat_MJPG, MFVideoFormat_NV12, MFVideoFormat_RGB32,
    MFVideoFormat_YUY2, MFSTARTUP_NOSOCKET, MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE,
    MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE, MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING,
    MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_VERSION,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

/// Media video format supported by a camera device.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraFormat {
    /// Horizontal resolution in pixels.
    pub width: u32,
    /// Vertical resolution in pixels.
    pub height: u32,
    /// Target frame rate in frames per second.
    pub frame_rate: f32,
    /// Subtype format encoding name (e.g. "NV12", "YUY2", "MJPG", "RGB32").
    pub format_name: String,
}

/// Metadata and capabilities of an attached camera or capture card.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraDeviceInfo {
    /// Device identifier / symbolic link string.
    pub id: String,
    /// Friendly user-facing device name (e.g. "Logitech BRIO", "Cam Link 4K").
    pub name: String,
    /// Supported resolution and framerate combinations.
    pub formats: Vec<CameraFormat>,
    /// Whether access is blocked by system privacy permissions.
    pub is_blocked: bool,
}

/// Converts standard BT.601 / BT.709 NV12 planar image to BGRA8.
pub fn nv12_to_bgra(nv12: &[u8], width: usize, height: usize, out_bgra: &mut [u8]) {
    let y_plane_size = width * height;
    if nv12.len() < y_plane_size + (y_plane_size / 2) || out_bgra.len() < width * height * 4 {
        return;
    }

    let y_plane = &nv12[..y_plane_size];
    let uv_plane = &nv12[y_plane_size..];

    for y in 0..height {
        let uv_row_start = (y / 2) * width;
        let y_row_start = y * width;
        let dst_row_start = y * width * 4;

        for x in 0..width {
            let y_val = y_plane[y_row_start + x] as f32;
            let uv_idx = uv_row_start + (x & !1);
            let u_val = uv_plane[uv_idx] as f32 - 128.0;
            let v_val = uv_plane[uv_idx + 1] as f32 - 128.0;

            let c = (y_val - 16.0).max(0.0) * 1.164383;
            let r = (c + 1.596027 * v_val).clamp(0.0, 255.0) as u8;
            let g = (c - 0.391762 * u_val - 0.812968 * v_val).clamp(0.0, 255.0) as u8;
            let b = (c + 2.017232 * u_val).clamp(0.0, 255.0) as u8;

            let px = dst_row_start + x * 4;
            out_bgra[px] = b;
            out_bgra[px + 1] = g;
            out_bgra[px + 2] = r;
            out_bgra[px + 3] = 255;
        }
    }
}

/// Converts YUY2 packed image (Y0 U0 Y1 V0) to BGRA8.
pub fn yuy2_to_bgra(yuy2: &[u8], width: usize, height: usize, out_bgra: &mut [u8]) {
    let expected_len = width * height * 2;
    if yuy2.len() < expected_len || out_bgra.len() < width * height * 4 {
        return;
    }

    for (chunk, out_quad) in yuy2.chunks_exact(4).zip(out_bgra.chunks_exact_mut(8)) {
        let y0 = chunk[0] as f32;
        let u = chunk[1] as f32 - 128.0;
        let y1 = chunk[2] as f32;
        let v = chunk[3] as f32 - 128.0;

        let c0 = (y0 - 16.0).max(0.0) * 1.164383;
        let r0 = (c0 + 1.596027 * v).clamp(0.0, 255.0) as u8;
        let g0 = (c0 - 0.391762 * u - 0.812968 * v).clamp(0.0, 255.0) as u8;
        let b0 = (c0 + 2.017232 * u).clamp(0.0, 255.0) as u8;

        out_quad[0] = b0;
        out_quad[1] = g0;
        out_quad[2] = r0;
        out_quad[3] = 255;

        let c1 = (y1 - 16.0).max(0.0) * 1.164383;
        let r1 = (c1 + 1.596027 * v).clamp(0.0, 255.0) as u8;
        let g1 = (c1 - 0.391762 * u - 0.812968 * v).clamp(0.0, 255.0) as u8;
        let b1 = (c1 + 2.017232 * u).clamp(0.0, 255.0) as u8;

        out_quad[4] = b1;
        out_quad[5] = g1;
        out_quad[6] = r1;
        out_quad[7] = 255;
    }
}

/// Enumerates all connected UVC webcams and capture card input devices via Media Foundation.
pub fn enumerate_camera_devices() -> Vec<CameraDeviceInfo> {
    // SAFETY: COM initialization for Media Foundation querying.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET).is_err() {
            return Vec::new();
        }
    }

    let mut devices = Vec::new();

    // SAFETY: Initializing Media Foundation attributes structure.
    unsafe {
        let mut attributes: Option<IMFAttributes> = None;
        if MFCreateAttributes(&mut attributes, 1).is_ok() {
            if let Some(attr) = attributes {
                let _ = attr.SetGUID(
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
                );

                let mut activate_ptrs: *mut Option<IMFActivate> = null_mut();
                let mut count = 0u32;

                if MFEnumDeviceSources(&attr, &mut activate_ptrs, &mut count).is_ok()
                    && !activate_ptrs.is_null()
                {
                    let activates = std::slice::from_raw_parts(activate_ptrs, count as usize);

                    for activate in activates.iter().flatten() {
                        let mut name_buf = [0u16; 512];
                        let mut name_len = 0u32;
                        let name = if activate
                            .GetString(
                                &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                                &mut name_buf,
                                Some(&mut name_len),
                            )
                            .is_ok()
                        {
                            String::from_utf16_lossy(&name_buf[..name_len as usize])
                        } else {
                            "Generic Video Capture Device".to_string()
                        };

                        let mut id_buf = [0u16; 1024];
                        let mut id_len = 0u32;
                        let id = if activate
                            .GetString(
                                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                                &mut id_buf,
                                Some(&mut id_len),
                            )
                            .is_ok()
                        {
                            String::from_utf16_lossy(&id_buf[..id_len as usize])
                        } else {
                            name.clone()
                        };

                        let mut formats = Vec::new();
                        let mut is_blocked = false;

                        // Query real native media formats supported by the device
                        match activate.ActivateObject::<IMFMediaSource>() {
                            Ok(media_source) => {
                                if let Ok(reader) =
                                    MFCreateSourceReaderFromMediaSource(&media_source, None)
                                {
                                    let mut idx = 0u32;
                                    while let Ok(native_type) = reader.GetNativeMediaType(
                                        MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                                        idx,
                                    ) {
                                        idx += 1;
                                        let subtype_str =
                                            if let Ok(st) = native_type.GetGUID(&MF_MT_SUBTYPE) {
                                                if st == MFVideoFormat_NV12 {
                                                    "NV12".to_string()
                                                } else if st == MFVideoFormat_YUY2 {
                                                    "YUY2".to_string()
                                                } else if st == MFVideoFormat_RGB32 {
                                                    "RGB32".to_string()
                                                } else if st == MFVideoFormat_MJPG {
                                                    "MJPG".to_string()
                                                } else {
                                                    format!("{:?}", st)
                                                }
                                            } else {
                                                "Unknown".to_string()
                                            };

                                        let mut w = 0u32;
                                        let mut h = 0u32;
                                        if let Ok(size) = native_type.GetUINT64(&MF_MT_FRAME_SIZE) {
                                            w = (size >> 32) as u32;
                                            h = (size & 0xFFFFFFFF) as u32;
                                        }

                                        let mut fps = 30.0f32;
                                        if let Ok(rate) = native_type.GetUINT64(&MF_MT_FRAME_RATE) {
                                            let num = (rate >> 32) as f32;
                                            let den = (rate & 0xFFFFFFFF) as f32;
                                            if den > 0.0 {
                                                fps = num / den;
                                            }
                                        }

                                        if w > 0 && h > 0 {
                                            let fmt = CameraFormat {
                                                width: w,
                                                height: h,
                                                frame_rate: (fps * 100.0).round() / 100.0,
                                                format_name: subtype_str,
                                            };
                                            if !formats.iter().any(|f: &CameraFormat| {
                                                f.width == fmt.width
                                                    && f.height == fmt.height
                                                    && (f.frame_rate - fmt.frame_rate).abs() < 0.1
                                                    && f.format_name == fmt.format_name
                                            }) {
                                                formats.push(fmt);
                                            }
                                        }
                                    }
                                }
                                let _ = media_source.Shutdown();
                            }
                            Err(err) => {
                                let code = err.code().0;
                                if code == windows::Win32::Foundation::E_ACCESSDENIED.0 {
                                    is_blocked = true;
                                    tracing::warn!(
                                        "[Camera] Access denied by Windows Privacy for device '{}'",
                                        name
                                    );
                                }
                            }
                        }

                        // Provide fallback formats if hardware format query was blocked or returned empty
                        if formats.is_empty() {
                            formats = vec![
                                CameraFormat {
                                    width: 1920,
                                    height: 1080,
                                    frame_rate: 60.0,
                                    format_name: "NV12".to_string(),
                                },
                                CameraFormat {
                                    width: 1920,
                                    height: 1080,
                                    frame_rate: 30.0,
                                    format_name: "NV12".to_string(),
                                },
                                CameraFormat {
                                    width: 1280,
                                    height: 720,
                                    frame_rate: 60.0,
                                    format_name: "NV12".to_string(),
                                },
                            ];
                        }

                        devices.push(CameraDeviceInfo {
                            id,
                            name,
                            formats,
                            is_blocked,
                        });
                    }

                    windows::Win32::System::Com::CoTaskMemFree(Some(activate_ptrs as *mut _));
                }
            }
        }

        let _ = MFShutdown();
        CoUninitialize();
    }

    devices
}

/// Asynchronous Media Foundation camera and capture card capture session.
pub struct MfCameraSession {
    device_id: String,
    active: Arc<AtomicBool>,
    permission_denied: Arc<AtomicBool>,
    error_message: Arc<parking_lot::Mutex<Option<String>>>,
    latest_frame: Arc<parking_lot::Mutex<Option<Arc<CapturedFrame>>>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl MfCameraSession {
    /// Creates and starts an asynchronous Media Foundation camera capture session.
    pub fn new(
        device_id_or_name: &str,
        target_width: u32,
        target_height: u32,
        target_format: &str,
    ) -> Result<Self, String> {
        let active = Arc::new(AtomicBool::new(true));
        let permission_denied = Arc::new(AtomicBool::new(false));
        let error_message = Arc::new(parking_lot::Mutex::new(None));
        let latest_frame = Arc::new(parking_lot::Mutex::new(None));

        let device_id = device_id_or_name.to_string();
        let target_format_str = target_format.to_string();

        let thread_active = Arc::clone(&active);
        let thread_perm = Arc::clone(&permission_denied);
        let thread_err = Arc::clone(&error_message);
        let thread_frame = Arc::clone(&latest_frame);
        let dev_id_clone = device_id.clone();

        let handle = std::thread::Builder::new()
            .name("mf-camera-worker".to_string())
            .spawn(move || {
                run_camera_worker(
                    dev_id_clone,
                    target_width,
                    target_height,
                    target_format_str,
                    thread_active,
                    thread_perm,
                    thread_err,
                    thread_frame,
                );
            })
            .map_err(|e| format!("Failed to spawn camera thread: {}", e))?;

        Ok(Self {
            device_id,
            active,
            permission_denied,
            error_message,
            latest_frame,
            thread_handle: Some(handle),
        })
    }

    /// Queries the latest captured frame from the background thread without blocking.
    pub fn get_latest_frame(&self) -> Option<Arc<CapturedFrame>> {
        self.latest_frame.lock().clone()
    }

    /// Returns `true` if camera access was blocked by Windows Privacy settings.
    pub fn is_permission_denied(&self) -> bool {
        self.permission_denied.load(Ordering::Relaxed)
    }

    /// Returns any internal driver or initialization error string.
    pub fn error_message(&self) -> Option<String> {
        self.error_message.lock().clone()
    }

    /// Returns the target device identifier.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Stops and terminates the capture thread.
    pub fn stop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for MfCameraSession {
    fn drop(&mut self) {
        self.stop();
    }
}

#[allow(clippy::too_many_arguments)]
fn run_camera_worker(
    target_id: String,
    target_width: u32,
    target_height: u32,
    _target_format: String,
    active: Arc<AtomicBool>,
    permission_denied: Arc<AtomicBool>,
    error_message: Arc<parking_lot::Mutex<Option<String>>>,
    latest_frame: Arc<parking_lot::Mutex<Option<Arc<CapturedFrame>>>>,
) {
    // SAFETY: COM initialization for worker thread.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET).is_err() {
            *error_message.lock() = Some("MFStartup failed on worker thread".to_string());
            return;
        }
    }

    // 1. Find matching IMFActivate for the requested camera
    let mut target_activate: Option<IMFActivate> = None;
    unsafe {
        let mut attributes: Option<IMFAttributes> = None;
        if MFCreateAttributes(&mut attributes, 1).is_ok() {
            if let Some(attr) = attributes {
                let _ = attr.SetGUID(
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
                );

                let mut activate_ptrs: *mut Option<IMFActivate> = null_mut();
                let mut count = 0u32;
                if MFEnumDeviceSources(&attr, &mut activate_ptrs, &mut count).is_ok()
                    && !activate_ptrs.is_null()
                {
                    let activates = std::slice::from_raw_parts(activate_ptrs, count as usize);
                    for act in activates.iter().flatten() {
                        let mut id_buf = [0u16; 1024];
                        let mut id_len = 0u32;
                        let id = if act
                            .GetString(
                                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                                &mut id_buf,
                                Some(&mut id_len),
                            )
                            .is_ok()
                        {
                            String::from_utf16_lossy(&id_buf[..id_len as usize])
                        } else {
                            String::new()
                        };

                        let mut name_buf = [0u16; 512];
                        let mut name_len = 0u32;
                        let name = if act
                            .GetString(
                                &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                                &mut name_buf,
                                Some(&mut name_len),
                            )
                            .is_ok()
                        {
                            String::from_utf16_lossy(&name_buf[..name_len as usize])
                        } else {
                            String::new()
                        };

                        if target_id.is_empty()
                            || target_id == "default"
                            || id == target_id
                            || name == target_id
                        {
                            target_activate = Some(act.clone());
                            break;
                        }
                    }
                    windows::Win32::System::Com::CoTaskMemFree(Some(activate_ptrs as *mut _));
                }
            }
        }
    }

    let Some(activate) = target_activate else {
        *error_message.lock() = Some("Target camera device not found".to_string());
        unsafe {
            let _ = MFShutdown();
            CoUninitialize();
        }
        return;
    };

    // 2. Activate IMFMediaSource
    let media_source: IMFMediaSource = match unsafe { activate.ActivateObject() } {
        Ok(src) => src,
        Err(err) => {
            let hr = err.code().0;
            tracing::error!(
                "[Camera] ActivateObject failed with HRESULT 0x{:08X}: {}",
                hr,
                err
            );
            if hr == windows::Win32::Foundation::E_ACCESSDENIED.0 {
                permission_denied.store(true, Ordering::Relaxed);
                *error_message.lock() = Some(
                    "Camera access denied by Windows Privacy settings. Enable camera access in Windows Settings -> Privacy & security -> Camera.".to_string(),
                );
            } else {
                *error_message.lock() = format!("Camera activation failed: {}", err).into();
            }
            unsafe {
                let _ = MFShutdown();
                CoUninitialize();
            }
            return;
        }
    };

    // 3. Create SourceReader with video processing enabled (for hardware YUV/MJPG -> RGB32 conversion)
    let mut reader_attr: Option<IMFAttributes> = None;
    let _ = unsafe { MFCreateAttributes(&mut reader_attr, 2) };
    if let Some(ref rattr) = reader_attr {
        let _ = unsafe { rattr.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1) };
    }

    let reader: IMFSourceReader =
        match unsafe { MFCreateSourceReaderFromMediaSource(&media_source, reader_attr.as_ref()) } {
            Ok(r) => r,
            Err(e) => {
                *error_message.lock() =
                    format!("MFCreateSourceReaderFromMediaSource failed: {}", e).into();
                unsafe {
                    let _ = media_source.Shutdown();
                    let _ = MFShutdown();
                    CoUninitialize();
                }
                return;
            }
        };

    // 4. Negotiate stream format (try RGB32 first, fallback to native format)
    let mut format_is_rgb32 = false;
    let mut format_is_nv12 = false;
    let mut format_is_yuy2 = false;

    if let Ok(out_type) = unsafe { MFCreateMediaType() } {
        let _ = unsafe { out_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video) };
        let _ = unsafe { out_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32) };
        if target_width > 0 && target_height > 0 {
            let size = ((target_width as u64) << 32) | (target_height as u64);
            let _ = unsafe { out_type.SetUINT64(&MF_MT_FRAME_SIZE, size) };
        }
        if unsafe {
            reader.SetCurrentMediaType(
                MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                None,
                &out_type,
            )
        }
        .is_ok()
        {
            format_is_rgb32 = true;
            tracing::info!("[Camera] Negotiated stream format: RGB32 (video processor)");
        }
    }

    if !format_is_rgb32 {
        if let Ok(cur_type) =
            unsafe { reader.GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32) }
        {
            if let Ok(subtype) = unsafe { cur_type.GetGUID(&MF_MT_SUBTYPE) } {
                if subtype == MFVideoFormat_RGB32 {
                    format_is_rgb32 = true;
                } else if subtype == MFVideoFormat_NV12 {
                    format_is_nv12 = true;
                } else if subtype == MFVideoFormat_YUY2 {
                    format_is_yuy2 = true;
                }
                tracing::info!("[Camera] Using native stream subtype: {:?}", subtype);
            }
        }
    }

    // Determine actual dimensions
    let (mut actual_w, mut actual_h) = (target_width.max(640), target_height.max(480));
    if let Ok(cur_type) =
        unsafe { reader.GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32) }
    {
        if let Ok(size) = unsafe { cur_type.GetUINT64(&MF_MT_FRAME_SIZE) } {
            let w = (size >> 32) as u32;
            let h = (size & 0xFFFFFFFF) as u32;
            if w > 0 && h > 0 {
                actual_w = w;
                actual_h = h;
            }
        }
    }
    tracing::info!(
        "[Camera] Ingest resolution: {}x{}, RGB32={}, NV12={}, YUY2={}",
        actual_w,
        actual_h,
        format_is_rgb32,
        format_is_nv12,
        format_is_yuy2
    );

    let mut sequence = 0u64;
    let mut bgra_buf = vec![0u8; (actual_w * actual_h * 4) as usize];

    // 5. ReadSample acquisition loop
    while active.load(Ordering::Relaxed) {
        let mut actual_stream_index = 0u32;
        let mut stream_flags = 0u32;
        let mut timestamp = 0i64;
        let mut sample: Option<IMFSample> = None;

        let hr = unsafe {
            reader.ReadSample(
                MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                0,
                Some(&mut actual_stream_index),
                Some(&mut stream_flags),
                Some(&mut timestamp),
                Some(&mut sample),
            )
        };

        if hr.is_err() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        let Some(sample) = sample else {
            std::thread::sleep(std::time::Duration::from_millis(2));
            continue;
        };

        let buffer = match unsafe { sample.ConvertToContiguousBuffer() } {
            Ok(b) => b,
            Err(_) => continue,
        };

        let mut ptr = null_mut();
        let mut max_len = 0u32;
        let mut cur_len = 0u32;
        if unsafe { buffer.Lock(&mut ptr, Some(&mut max_len), Some(&mut cur_len)) }.is_err() {
            continue;
        }

        if !ptr.is_null() && cur_len > 0 {
            let sample_bytes = unsafe { std::slice::from_raw_parts(ptr, cur_len as usize) };
            if format_is_rgb32 {
                if bgra_buf.len() != sample_bytes.len() {
                    bgra_buf.resize(sample_bytes.len(), 255);
                }
                bgra_buf.copy_from_slice(sample_bytes);
            } else if format_is_nv12 {
                nv12_to_bgra(
                    sample_bytes,
                    actual_w as usize,
                    actual_h as usize,
                    &mut bgra_buf,
                );
            } else if format_is_yuy2 {
                yuy2_to_bgra(
                    sample_bytes,
                    actual_w as usize,
                    actual_h as usize,
                    &mut bgra_buf,
                );
            } else {
                if bgra_buf.len() < (actual_w * actual_h * 4) as usize {
                    bgra_buf.resize((actual_w * actual_h * 4) as usize, 255);
                }
                let copy_len = bgra_buf.len().min(sample_bytes.len());
                bgra_buf[..copy_len].copy_from_slice(&sample_bytes[..copy_len]);
            }

            let _ = unsafe { buffer.Unlock() };

            sequence += 1;
            let now_sec = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);

            *latest_frame.lock() = Some(Arc::new(CapturedFrame {
                width: actual_w,
                height: actual_h,
                rgba: Arc::new(bgra_buf.clone()),
                timestamp_seconds: now_sec,
                sequence,
                is_bgra: true,
            }));

            tracing::trace!(sequence, "[Camera] Received and processed frame");
        } else {
            let _ = unsafe { buffer.Unlock() };
        }
    }

    unsafe {
        let _ = media_source.Shutdown();
        let _ = MFShutdown();
        CoUninitialize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_device_enumeration_does_not_panic() {
        let devices = enumerate_camera_devices();
        // May be empty on headless VMs/CI or contain connected webcams
        for dev in &devices {
            assert!(!dev.name.is_empty());
            assert!(!dev.id.is_empty());
            assert!(!dev.formats.is_empty());
        }
    }

    #[test]
    fn test_nv12_to_bgra_conversion_dimensions_and_channels() {
        let width = 4usize;
        let height = 4usize;
        // 4x4 NV12: Y plane = 16 bytes, UV plane = 8 bytes (total 24 bytes)
        // Set Y=235 (nominal white), U=128 (neutral), V=128 (neutral)
        let mut nv12 = vec![235u8; 16];
        nv12.extend_from_slice(&[128u8; 8]);

        let mut bgra = vec![0u8; width * height * 4];
        nv12_to_bgra(&nv12, width, height, &mut bgra);

        // Expect near-white (255, 255, 255, 255) in BGRA
        for chunk in bgra.chunks_exact(4) {
            assert!(chunk[0] >= 240, "Blue should be bright: {}", chunk[0]);
            assert!(chunk[1] >= 240, "Green should be bright: {}", chunk[1]);
            assert!(chunk[2] >= 240, "Red should be bright: {}", chunk[2]);
            assert_eq!(chunk[3], 255, "Alpha must be 255");
        }
    }

    #[test]
    fn test_yuy2_to_bgra_conversion_dimensions_and_channels() {
        let width = 4usize;
        let height = 2usize;
        // 4x2 YUY2: width * height * 2 = 16 bytes
        // Y0=235, U=128, Y1=235, V=128 -> White pixels
        let yuy2 = vec![
            235, 128, 235, 128, 235, 128, 235, 128, 235, 128, 235, 128, 235, 128, 235, 128,
        ];
        let mut bgra = vec![0u8; width * height * 4];
        yuy2_to_bgra(&yuy2, width, height, &mut bgra);

        for chunk in bgra.chunks_exact(4) {
            assert!(chunk[0] >= 240, "Blue: {}", chunk[0]);
            assert!(chunk[1] >= 240, "Green: {}", chunk[1]);
            assert!(chunk[2] >= 240, "Red: {}", chunk[2]);
            assert_eq!(chunk[3], 255);
        }
    }

    #[test]
    fn test_mf_camera_session_graceful_handling_of_invalid_device() {
        let session =
            MfCameraSession::new("non_existent_camera_device_uuid_9999", 1280, 720, "NV12");
        assert!(session.is_ok());
        let s = session.unwrap();
        // Give background thread a moment to attempt lookup and terminate gracefully
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(s.get_latest_frame().is_none());
    }
}
