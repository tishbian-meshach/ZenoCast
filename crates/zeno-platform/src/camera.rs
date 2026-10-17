//! Windows Media Foundation camera capture and device enumeration.

use std::ptr::null_mut;
use windows::Win32::Media::MediaFoundation::{
    IMFActivate, IMFAttributes, MFCreateAttributes, MFEnumDeviceSources, MFShutdown, MFStartup,
    MFSTARTUP_NOSOCKET, MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME, MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK, MF_VERSION,
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

                        // Common standard broadcast fallback formats
                        let formats = vec![
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

                        devices.push(CameraDeviceInfo { id, name, formats });
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
