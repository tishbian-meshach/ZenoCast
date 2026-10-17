//! WASAPI audio hardware device enumeration and capture initialization via CPAL.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

/// Audio device metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    /// Device identifier or name.
    pub name: String,
    /// Whether this is the system default communication / multimedia device.
    pub is_default: bool,
    /// Default hardware sample rate in Hz.
    pub default_sample_rate: u32,
    /// Maximum supported channel count.
    pub channels: u16,
}

/// Enumerates all available WASAPI physical microphone and loopback audio devices.
pub fn enumerate_audio_devices() -> (Vec<AudioDeviceInfo>, Vec<AudioDeviceInfo>) {
    let host = cpal::default_host();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    let default_in = host.default_input_device().and_then(|d| d.name().ok());
    let default_out = host.default_output_device().and_then(|d| d.name().ok());

    if let Ok(devices) = host.input_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                let default_cfg = device.default_input_config().ok();
                let sample_rate = default_cfg
                    .as_ref()
                    .map(|c| c.sample_rate().0)
                    .unwrap_or(48000);
                let channels = default_cfg.as_ref().map(|c| c.channels()).unwrap_or(2);
                let is_default = default_in.as_deref() == Some(&name);

                inputs.push(AudioDeviceInfo {
                    name,
                    is_default,
                    default_sample_rate: sample_rate,
                    channels,
                });
            }
        }
    }

    if let Ok(devices) = host.output_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                let default_cfg = device.default_output_config().ok();
                let sample_rate = default_cfg
                    .as_ref()
                    .map(|c| c.sample_rate().0)
                    .unwrap_or(48000);
                let channels = default_cfg.as_ref().map(|c| c.channels()).unwrap_or(2);
                let is_default = default_out.as_deref() == Some(&name);

                outputs.push(AudioDeviceInfo {
                    name,
                    is_default,
                    default_sample_rate: sample_rate,
                    channels,
                });
            }
        }
    }

    (inputs, outputs)
}
