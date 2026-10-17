//! Unified hardware capture device discovery, window scanning, and camera hot-plug monitoring.

use std::collections::HashMap;
use zeno_platform::{
    enumerate_camera_devices, enumerate_displays, enumerate_windows, CameraDeviceInfo, DisplayInfo,
    WindowInfo,
};

/// Hot-plug connection notification for external hardware capture devices.
#[derive(Debug, Clone, PartialEq)]
pub enum HotplugEvent {
    /// A new camera or capture card was plugged into the system.
    DeviceConnected(CameraDeviceInfo),
    /// A previously connected camera was unplugged or disconnected.
    DeviceDisconnected {
        /// Symbolic link / device identifier of the disconnected device.
        device_id: String,
        /// Name of the disconnected device.
        name: String,
    },
}

/// Unified manager for discovering capture targets and tracking hot-plug events.
pub struct DeviceManager {
    known_cameras: HashMap<String, CameraDeviceInfo>,
}

impl DeviceManager {
    /// Initialize a new device manager.
    pub fn new() -> Self {
        let initial_cams = enumerate_camera_devices();
        let mut known = HashMap::new();
        for cam in initial_cams {
            known.insert(cam.id.clone(), cam);
        }

        Self {
            known_cameras: known,
        }
    }

    /// Enumerates currently active physical display monitors.
    pub fn list_displays(&self) -> Vec<DisplayInfo> {
        enumerate_displays()
    }

    /// Enumerates visible application windows eligible for window capture.
    pub fn list_windows(&self) -> Vec<WindowInfo> {
        enumerate_windows()
    }

    /// Enumerates currently available camera and capture card devices.
    pub fn list_cameras(&self) -> Vec<CameraDeviceInfo> {
        enumerate_camera_devices()
    }

    /// Polls for hardware device arrival or removal since the last check.
    pub fn poll_hotplug_events(&mut self) -> Vec<HotplugEvent> {
        let current_cams = self.list_cameras();
        let mut current_map = HashMap::new();
        let mut events = Vec::new();

        for cam in current_cams {
            current_map.insert(cam.id.clone(), cam);
        }

        // Detect disconnected devices
        for (id, old_cam) in &self.known_cameras {
            if !current_map.contains_key(id) {
                events.push(HotplugEvent::DeviceDisconnected {
                    device_id: id.clone(),
                    name: old_cam.name.clone(),
                });
            }
        }

        // Detect newly connected devices
        for (id, new_cam) in &current_map {
            if !self.known_cameras.contains_key(id) {
                events.push(HotplugEvent::DeviceConnected(new_cam.clone()));
            }
        }

        self.known_cameras = current_map;
        events
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}
