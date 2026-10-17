//! GPU hardware adapter, device, and queue initialization.

use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    MemoryHints, PowerPreference, Queue, RequestAdapterOptions,
};

/// Initialized GPU context containing Device, Queue, and Adapter metadata.
pub struct GpuContext {
    /// Wgpu instance.
    pub instance: Instance,
    /// Physical or virtual graphics adapter.
    pub adapter: Adapter,
    /// Logical device handle.
    pub device: Device,
    /// Command execution queue.
    pub queue: Queue,
}

impl GpuContext {
    /// Initialize a headless GPU context suitable for offline rendering and testing.
    /// Prefers high-performance discrete GPU, with automatic software fallback.
    pub async fn init_headless() -> Result<Self, String> {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "Failed to find suitable GPU adapter".to_string())?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("ZenocastHeadlessDevice"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to request logical GPU device: {}", e))?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}
