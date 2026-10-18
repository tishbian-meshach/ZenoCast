//! Audio input (microphone/line-in) and audio output (loopback) broadcast capture sources.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, SourceProperties, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use wgpu::{Device, Queue};
use zeno_core::SourceId;

/// Audio capture source for hardware microphones, line inputs, and desktop loopbacks.
pub struct AudioCaptureSource {
    id: SourceId,
    name: String,
    device_name: String,
    is_loopback: bool,
    volume: f32,
    muted: bool,
    level: f32,
    width: u32,
    height: u32,
    active: bool,
    healthy: bool,
    last_update_time: f64,
    texture_buffer: DoubleBufferedTexture,
    raw_buffer: Vec<u8>,
}

impl AudioCaptureSource {
    /// Creates a microphone or physical line-in audio input source.
    pub fn new_input(name: impl Into<String>, device_name: impl Into<String>) -> Self {
        Self {
            id: SourceId::new(),
            name: name.into(),
            device_name: device_name.into(),
            is_loopback: false,
            volume: 1.0,
            muted: false,
            level: 0.0,
            width: 640,
            height: 120,
            active: false,
            healthy: true,
            last_update_time: -1.0,
            texture_buffer: DoubleBufferedTexture::new(),
            raw_buffer: Vec::new(),
        }
    }

    /// Creates a system or application audio output loopback capture source.
    pub fn new_output(name: impl Into<String>, device_name: impl Into<String>) -> Self {
        Self {
            id: SourceId::new(),
            name: name.into(),
            device_name: device_name.into(),
            is_loopback: true,
            volume: 1.0,
            muted: false,
            level: 0.0,
            width: 640,
            height: 120,
            active: false,
            healthy: true,
            last_update_time: -1.0,
            texture_buffer: DoubleBufferedTexture::new(),
            raw_buffer: Vec::new(),
        }
    }

    /// Set explicit source ID.
    pub fn with_id(mut self, id: SourceId) -> Self {
        self.id = id;
        self
    }

    /// Sets audio gain volume multiplier [0.0..1.0].
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Returns audio gain volume multiplier.
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Sets mute state.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Returns `true` if muted.
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Current peak meter level [0.0..1.0].
    pub fn meter_level(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.level * self.volume
        }
    }

    /// Render visual stereo audio meter onto RGBA buffer.
    fn render_meter(&mut self, current_time: f64) {
        let buf_size = (self.width * self.height * 4) as usize;
        if self.raw_buffer.len() < buf_size {
            self.raw_buffer.resize(buf_size, 0);
        }

        // Simulate dynamic speech / sound level oscillations when active
        if !self.muted {
            let osc = ((current_time * 8.0).sin().abs() * 0.7
                + (current_time * 3.3).cos().abs() * 0.3) as f32;
            self.level = osc.clamp(0.05, 0.95);
        } else {
            self.level = 0.0;
        }

        let effective_level = self.meter_level();
        let bar_width = ((self.width as f32 - 40.0) * effective_level) as u32;

        for y in 0..self.height {
            let row_offset = (y * self.width * 4) as usize;
            let in_meter_bar = (40..=80).contains(&y);

            for x in 0..self.width {
                let px = row_offset + (x * 4) as usize;
                if in_meter_bar && x >= 20 && x <= self.width - 20 {
                    let meter_x = x - 20;
                    if meter_x <= bar_width {
                        // Broadcast audio level coloring: safe blue/neutral into yellow/red peak
                        let pct = meter_x as f32 / (self.width - 40) as f32;
                        if pct > 0.85 {
                            self.raw_buffer[px] = 255; // Red peak #FF3B30
                            self.raw_buffer[px + 1] = 59;
                            self.raw_buffer[px + 2] = 48;
                        } else if pct > 0.70 {
                            self.raw_buffer[px] = 255; // Yellow warning #FFD60A
                            self.raw_buffer[px + 1] = 214;
                            self.raw_buffer[px + 2] = 10;
                        } else {
                            self.raw_buffer[px] = 76; // Blue/green active
                            self.raw_buffer[px + 1] = 141;
                            self.raw_buffer[px + 2] = 255;
                        }
                        self.raw_buffer[px + 3] = 255;
                    } else {
                        // Empty meter track
                        self.raw_buffer[px] = 39; // Neutral #272B30
                        self.raw_buffer[px + 1] = 43;
                        self.raw_buffer[px + 2] = 48;
                        self.raw_buffer[px + 3] = 255;
                    }
                } else if x == 0 || x == self.width - 1 || y == 0 || y == self.height - 1 {
                    self.raw_buffer[px] = 39;
                    self.raw_buffer[px + 1] = 43;
                    self.raw_buffer[px + 2] = 48;
                    self.raw_buffer[px + 3] = 255;
                } else {
                    self.raw_buffer[px] = 21; // Neutral background #15171A
                    self.raw_buffer[px + 1] = 23;
                    self.raw_buffer[px + 2] = 26;
                    self.raw_buffer[px + 3] = 255;
                }
            }
        }
    }
}

impl Source for AudioCaptureSource {
    fn id(&self) -> SourceId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn kind(&self) -> SourceKind {
        if self.is_loopback {
            SourceKind::AudioOutput
        } else {
            SourceKind::AudioInput
        }
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn is_healthy(&self) -> bool {
        self.active && self.healthy
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn properties(&self) -> SourceProperties {
        if self.is_loopback {
            SourceProperties::AudioOutput {
                device_name: self.device_name.clone(),
                volume: self.volume,
                muted: self.muted,
            }
        } else {
            SourceProperties::AudioInput {
                device_name: self.device_name.clone(),
                volume: self.volume,
                muted: self.muted,
            }
        }
    }

    fn update_properties(
        &mut self,
        properties: &SourceProperties,
        _device: &Device,
        _queue: &Queue,
    ) -> Result<(), SourceError> {
        match properties {
            SourceProperties::AudioInput {
                device_name,
                volume,
                muted,
            } => {
                self.device_name = device_name.clone();
                self.volume = *volume;
                self.muted = *muted;
                Ok(())
            }
            SourceProperties::AudioOutput {
                device_name,
                volume,
                muted,
            } => {
                self.device_name = device_name.clone();
                self.volume = *volume;
                self.muted = *muted;
                Ok(())
            }
            _ => Err(SourceError::InvalidProperties(
                "Expected AudioInput or AudioOutput properties".to_string(),
            )),
        }
    }

    fn start(&mut self, device: &Device, queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.last_update_time = -1.0;
        self.render_meter(0.0);
        self.texture_buffer.upload(
            device,
            queue,
            self.width,
            self.height,
            &self.raw_buffer,
            0.0,
        );
        Ok(())
    }

    fn stop(&mut self) {
        self.active = false;
    }

    fn get_frame(&mut self) -> Option<&VideoFrame> {
        self.texture_buffer.current_frame()
    }

    fn update(&mut self, device: &Device, queue: &Queue, current_time: f64) {
        if !self.active {
            return;
        }

        if self.last_update_time >= 0.0 && (current_time - self.last_update_time) < (1.0 / 60.0) {
            return;
        }

        self.render_meter(current_time);
        self.texture_buffer.upload(
            device,
            queue,
            self.width,
            self.height,
            &self.raw_buffer,
            current_time,
        );
        self.last_update_time = current_time;
    }
}
