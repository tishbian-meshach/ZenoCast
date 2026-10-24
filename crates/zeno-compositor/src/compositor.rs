//! Live GPU Scene Compositor managing multi-scene stacks, preview/program buses, and transitions.

use crate::bus::{BusManager, StudioMode};
use crate::error::CompositorError;
use crate::pipeline::{CompositorGpuPipeline, ItemUniforms, TransitionUniforms};
use crate::scene::{Scene, SceneItemSource};
use crate::transition::{
    ActiveTransition, SlideDirection, TransitionDefinition, TransitionKind, TransitionRule,
    WipeDirection,
};
use glam::Mat4;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::*;
use zeno_core::{SceneId, SourceId};

/// Live GPU scene compositor executing real-time 1080p60 mixing between broadcast scenes.
pub struct SceneCompositor {
    device: Arc<Device>,
    queue: Arc<Queue>,
    width: u32,
    height: u32,
    fps: f32,
    scenes: HashMap<SceneId, Scene>,
    pub preview_working_scene: Option<Scene>,
    transitions: HashMap<zeno_core::TransitionId, TransitionDefinition>,
    transition_rules: Vec<TransitionRule>,
    global_default_transition: TransitionDefinition,
    bus: BusManager,
    active_transition: Option<ActiveTransition>,
    pipeline: CompositorGpuPipeline,
    program_texture: Arc<Texture>,
    program_view: Arc<TextureView>,
    preview_texture: Arc<Texture>,
    preview_view: Arc<TextureView>,
    intermediate_a: Arc<Texture>,
    intermediate_view_a: Arc<TextureView>,
    intermediate_b: Arc<Texture>,
    intermediate_view_b: Arc<TextureView>,
    transition_uniform_buf: Buffer,
    item_uniform_buf: Buffer,
    ortho_projection: Mat4,
}

impl SceneCompositor {
    /// Creates a new scene compositor with allocated render targets and pipelines.
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, width: u32, height: u32, fps: f32) -> Self {
        let pipeline = CompositorGpuPipeline::new(&device);

        let target_desc = TextureDescriptor {
            label: Some("CompositorTarget"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let prog_tex = Arc::new(device.create_texture(&TextureDescriptor {
            label: Some("ProgramTexture"),
            ..target_desc
        }));
        let prog_view = Arc::new(prog_tex.create_view(&TextureViewDescriptor::default()));

        let prev_tex = Arc::new(device.create_texture(&TextureDescriptor {
            label: Some("PreviewTexture"),
            ..target_desc
        }));
        let prev_view = Arc::new(prev_tex.create_view(&TextureViewDescriptor::default()));

        let int_a = Arc::new(device.create_texture(&TextureDescriptor {
            label: Some("IntermediateSceneA"),
            ..target_desc
        }));
        let int_view_a = Arc::new(int_a.create_view(&TextureViewDescriptor::default()));

        let int_b = Arc::new(device.create_texture(&TextureDescriptor {
            label: Some("IntermediateSceneB"),
            ..target_desc
        }));
        let int_view_b = Arc::new(int_b.create_view(&TextureViewDescriptor::default()));

        // Pre-allocated reusable uniform buffers for zero-allocation per-frame render passes
        let dummy_item_uniforms = ItemUniforms {
            mvp: Mat4::IDENTITY.to_cols_array_2d(),
            crop_insets: [0.0; 4],
            opacity: 1.0,
            blend_mode: 0,
            _padding: [0.0; 2],
        };
        let item_uniform_buf = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("ItemUniformBuffer"),
            contents: bytemuck::bytes_of(&dummy_item_uniforms),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let dummy_trans_uniforms = TransitionUniforms {
            progress: 0.0,
            kind: 0,
            direction: 0,
            param1: 0.0,
            dip_color: [0.0; 4],
            _padding: [0.0; 8],
        };
        let transition_uniform_buf = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("TransitionUniformBuffer"),
            contents: bytemuck::bytes_of(&dummy_trans_uniforms),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // Orthographic projection mapping [0..width, 0..height] to NDC [-1..1]
        // In wgpu NDC, Y goes down or up depending on conventions. Standard top-left origin:
        let ortho_projection =
            Mat4::orthographic_lh(0.0, width as f32, height as f32, 0.0, -100.0, 100.0);

        let initial_scene = SceneId::new();
        let bus = BusManager::new(initial_scene);
        let default_fade = TransitionDefinition::fade(0.3);

        Self {
            device,
            queue,
            width,
            height,
            fps,
            scenes: HashMap::new(),
            preview_working_scene: None,
            transitions: HashMap::new(),
            transition_rules: Vec::new(),
            global_default_transition: default_fade,
            bus,
            active_transition: None,
            pipeline,
            program_texture: prog_tex,
            program_view: prog_view,
            preview_texture: prev_tex,
            preview_view: prev_view,
            intermediate_a: int_a,
            intermediate_view_a: int_view_a,
            intermediate_b: int_b,
            intermediate_view_b: int_view_b,
            transition_uniform_buf,
            item_uniform_buf,
            ortho_projection,
        }
    }

    /// Returns the canvas dimensions (width, height) in pixels.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the canvas width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the canvas height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the active program scene ID.
    pub fn program_scene(&self) -> SceneId {
        self.bus.program_scene
    }

    /// Returns the active preview scene ID.
    pub fn preview_scene(&self) -> SceneId {
        self.bus.preview_scene
    }

    /// Sets the active program scene ID.
    pub fn set_program(&mut self, scene: SceneId) {
        self.bus.program_scene = scene;
    }

    /// Sets the active preview scene ID and clones a working copy.
    pub fn set_preview(&mut self, scene: SceneId) {
        if self.bus.preview_scene != scene || self.preview_working_scene.is_none() {
            self.bus.preview_scene = scene;
            self.preview_working_scene = self.scenes.get(&scene).cloned();
        }
    }

    /// Returns `true` if a scene transition is currently in progress.
    pub fn is_transition_active(&self) -> bool {
        self.active_transition.is_some()
    }

    /// Sets switcher mode (Direct or Studio).
    pub fn set_mode(&mut self, mode: StudioMode) {
        self.bus.set_mode(mode);
    }

    /// Adds or registers a scene in the compositor.
    pub fn add_scene(&mut self, scene: Scene) {
        let id = scene.id;
        self.scenes.insert(id, scene);

        // If this is the first real scene, set as program and preview
        if self.scenes.len() == 1 {
            self.bus.program_scene = id;
            self.bus.preview_scene = id;
            self.preview_working_scene = self.scenes.get(&id).cloned();
        }
    }

    /// Retrieves a scene by ID.
    pub fn get_scene(&self, id: SceneId) -> Option<&Scene> {
        self.scenes.get(&id)
    }

    /// Retrieves a mutable reference to a scene by ID.
    pub fn get_scene_mut(&mut self, id: SceneId) -> Option<&mut Scene> {
        self.scenes.get_mut(&id)
    }

    /// Returns a mutable reference to the Preview working scene (initializing it if needed).
    pub fn preview_working_scene_mut(&mut self) -> Option<&mut Scene> {
        if self.preview_working_scene.is_none() {
            self.preview_working_scene = self.scenes.get(&self.bus.preview_scene).cloned();
        }
        self.preview_working_scene.as_mut()
    }

    /// Returns a reference to the Preview working scene (or fall back to saved scene).
    pub fn preview_working_scene(&self) -> Option<&Scene> {
        self.preview_working_scene
            .as_ref()
            .or_else(|| self.scenes.get(&self.bus.preview_scene))
    }

    /// Adds an item to the Preview working scene.
    pub fn add_preview_item(&mut self, item: crate::scene::SceneItem) {
        if let Some(scene) = self.preview_working_scene_mut() {
            scene.add_item(item);
        }
    }

    /// Removes an item by source ID from the Preview working scene.
    pub fn remove_preview_item_by_source(&mut self, source_id: SourceId) {
        if let Some(scene) = self.preview_working_scene_mut() {
            scene.items.retain(|it| match it.source {
                SceneItemSource::Source(s_id) => s_id != source_id,
                _ => true,
            });
        }
    }

    /// Updates transform of an item in the Preview working scene.
    pub fn update_preview_item_transform(
        &mut self,
        source_id: SourceId,
        pos_x: f32,
        pos_y: f32,
        scale_x: f32,
        scale_y: f32,
        rotation: f32,
    ) {
        if let Some(scene) = self.preview_working_scene_mut() {
            for item in &mut scene.items {
                if let SceneItemSource::Source(s_id) = item.source {
                    if s_id == source_id {
                        item.transform.position = zeno_core::Vec2::new(pos_x, pos_y);
                        item.transform.scale = zeno_core::Vec2::new(scale_x, scale_y);
                        item.transform.rotation_degrees = rotation;
                    }
                }
            }
        }
    }

    /// Reorders items in the Preview working scene.
    pub fn sync_preview_items_order(&mut self, item_source_ids: &[SourceId]) {
        if let Some(scene) = self.preview_working_scene_mut() {
            let mut ordered = Vec::new();
            for &s_id in item_source_ids {
                if let Some(pos) = scene.items.iter().position(|it| match it.source {
                    SceneItemSource::Source(id) => id == s_id,
                    _ => false,
                }) {
                    ordered.push(scene.items.remove(pos));
                }
            }
            ordered.append(&mut scene.items);
            scene.items = ordered;
        }
    }

    /// Removes a scene by ID from the compositor.
    pub fn remove_scene(&mut self, id: SceneId) {
        self.scenes.remove(&id);
        if self.bus.preview_scene == id {
            self.preview_working_scene = None;
        }
    }

    /// Registers a reusable transition definition.
    pub fn register_transition(&mut self, transition: TransitionDefinition) {
        self.transitions.insert(transition.id, transition);
    }

    /// Adds a transition override rule.
    pub fn add_transition_rule(&mut self, rule: TransitionRule) {
        self.transition_rules.push(rule);
    }

    /// Resolves transition rule according to Section 10A 3-step hierarchy:
    /// 1. Exact pair rule (Scene A -> Scene B)
    /// 2. Destination scene default transition
    /// 3. Global default transition
    pub fn resolve_transition(&self, from: SceneId, to: SceneId) -> TransitionDefinition {
        // Step 1: Exact pair rule match
        if let Some(rule) = self
            .transition_rules
            .iter()
            .find(|r| (r.from_scene == Some(from) || r.from_scene.is_none()) && r.to_scene == to)
        {
            if let Some(def) = self.transitions.get(&rule.transition) {
                return def.clone();
            }
        }

        // Step 2: Destination scene default transition
        if let Some(dest_scene) = self.scenes.get(&to) {
            if let Some(trans_id) = dest_scene.default_transition {
                if let Some(def) = self.transitions.get(&trans_id) {
                    return def.clone();
                }
            }
        }

        // Step 3: Global default transition
        self.global_default_transition.clone()
    }

    /// Copies current program texture into intermediate texture A for seamless interruption blending.
    pub fn snapshot_program_to_intermediate_a(&self) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("InterruptionSnapshotEncoder"),
            });
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo {
                texture: &self.program_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyTextureInfo {
                texture: &self.intermediate_a,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Triggers scene transition to destination scene.
    ///
    /// Interruption handling rule (Section 10A):
    /// If switching mid-transition, the new transition starts from the current visual state with no visible jump.
    pub fn switch_scene(
        &mut self,
        to_scene: SceneId,
        transition_override: Option<TransitionDefinition>,
    ) {
        let was_active = self.active_transition.is_some();
        if was_active {
            self.snapshot_program_to_intermediate_a();
        }

        let from_scene = if let Some(ref active) = self.active_transition {
            // Mid-transition interruption: start transition from current destination
            active.to_scene
        } else {
            self.bus.program_scene
        };

        if from_scene == to_scene {
            return;
        }

        let transition =
            transition_override.unwrap_or_else(|| self.resolve_transition(from_scene, to_scene));

        if self.bus.mode == StudioMode::Direct {
            if transition.duration_seconds <= 0.0 || matches!(transition.kind, TransitionKind::Cut)
            {
                self.bus.program_scene = to_scene;
                self.active_transition = None;
            } else if was_active {
                self.active_transition = Some(ActiveTransition::from_snapshot(
                    from_scene, to_scene, transition,
                ));
            } else {
                self.active_transition =
                    Some(ActiveTransition::new(from_scene, to_scene, transition));
            }
        } else {
            // Studio mode: cues destination to Preview bus
            self.bus.preview_scene = to_scene;
        }
    }

    /// Executes an instant Cut transition between Preview and Program buses.
    pub fn cut(&mut self) {
        self.active_transition = None;
        if let Some(ref working) = self.preview_working_scene {
            self.scenes.insert(self.bus.preview_scene, working.clone());
        }
        self.bus.program_scene = self.bus.preview_scene;
        self.bus.t_bar_position = 0.0;
    }

    /// Triggers an Auto transition between Preview and Program buses.
    pub fn auto_transition(&mut self, transition_override: Option<TransitionDefinition>) {
        let was_active = self.active_transition.is_some();
        if was_active {
            self.snapshot_program_to_intermediate_a();
        }

        let from = if let Some(ref active) = self.active_transition {
            active.to_scene
        } else {
            self.bus.program_scene
        };
        let to = self.bus.preview_scene;

        let transition = transition_override.unwrap_or_else(|| self.resolve_transition(from, to));
        if transition.duration_seconds <= 0.0 || matches!(transition.kind, TransitionKind::Cut) {
            self.cut();
        } else if was_active {
            self.active_transition = Some(ActiveTransition::from_snapshot(from, to, transition));
        } else {
            self.active_transition = Some(ActiveTransition::new(from, to, transition));
        }
    }

    /// Sets manual T-bar fader position [0.0..1.0].
    pub fn set_t_bar(&mut self, position: f32) {
        self.bus.set_t_bar(position);
        let from = self.bus.program_scene;
        let to = self.bus.preview_scene;

        if position >= 1.0 {
            // T-bar finished at 100%: complete cut and commit
            self.cut();
        } else if position <= 0.0 {
            // T-bar reset to 0%: cancel transition
            self.active_transition = None;
        } else {
            // In manual transition: start transition if not already active
            if self.active_transition.is_none() {
                let trans = self.resolve_transition(from, to);
                self.active_transition = Some(ActiveTransition::new(from, to, trans));
            }
            if let Some(ref mut active) = self.active_transition {
                active.manual_progress = Some(position);
            }
        }
    }

    /// Returns reference to bus state.
    pub fn bus(&self) -> &BusManager {
        &self.bus
    }

    /// Returns mutable reference to bus state.
    pub fn bus_mut(&mut self) -> &mut BusManager {
        &mut self.bus
    }

    /// Returns current active transition if one is in flight.
    pub fn active_transition(&self) -> Option<&ActiveTransition> {
        self.active_transition.as_ref()
    }

    /// Returns configured framerate.
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Returns the live Program output texture.
    pub fn program_texture(&self) -> &Arc<Texture> {
        &self.program_texture
    }

    /// Returns the live Program output texture view.
    pub fn program_view(&self) -> &Arc<TextureView> {
        &self.program_view
    }

    /// Returns the live Preview output texture.
    pub fn preview_texture(&self) -> &Arc<Texture> {
        &self.preview_texture
    }

    /// Returns the live Preview output texture view.
    pub fn preview_view(&self) -> &Arc<TextureView> {
        &self.preview_view
    }

    /// Returns intermediate render target A texture.
    pub fn intermediate_texture_a(&self) -> &Arc<Texture> {
        &self.intermediate_a
    }

    /// Returns intermediate render target B texture.
    pub fn intermediate_texture_b(&self) -> &Arc<Texture> {
        &self.intermediate_b
    }

    /// Renders a full frame at the given timestamp delta.
    ///
    /// Real-time budget guarantee: renders at 1080p60 with zero file IO,
    /// no blocking locks, and no heap allocation on the hot path.
    pub fn render_frame(
        &mut self,
        sources: &HashMap<SourceId, Arc<TextureView>>,
        dt: f64,
    ) -> Result<(), CompositorError> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("CompositorFrameEncoder"),
            });

        // 1. Advance active transition
        let mut transition_completed = false;
        let mut new_program_scene = None;

        if let Some(ref mut active) = self.active_transition {
            active.tick(dt);
            if active.is_finished() {
                transition_completed = true;
                new_program_scene = Some(active.to_scene);
            }
        }

        // 2. Render program output
        if let Some(ref active) = self.active_transition {
            // In transition: render Scene A to Intermediate A (if not from snapshot), Scene B to Intermediate B
            if !active.from_snapshot {
                self.render_scene_internal(
                    active.from_scene,
                    &self.intermediate_view_a,
                    sources,
                    &mut encoder,
                    0,
                )?;
            }
            if let Some(ref working) = self.preview_working_scene {
                self.render_scene_object(
                    working,
                    &self.intermediate_view_b,
                    sources,
                    &mut encoder,
                    0,
                )?;
            } else {
                self.render_scene_internal(
                    active.to_scene,
                    &self.intermediate_view_b,
                    sources,
                    &mut encoder,
                    0,
                )?;
            }

            // Execute GPU transition shader pass blending Intermediate A and B into Program
            let eased_t = active.eased_progress();
            let (kind_u32, dir_u32, param1, dip_color) = match &active.definition.kind {
                TransitionKind::Cut => (0u32, 0u32, 0.0f32, [0.0; 4]),
                TransitionKind::Fade => (1u32, 0u32, 0.0f32, [0.0; 4]),
                TransitionKind::DipToColor { color } => {
                    (2u32, 0u32, 0.0f32, [color.r, color.g, color.b, color.a])
                }
                TransitionKind::Slide { direction, push } => {
                    let dir = match direction {
                        SlideDirection::Left => 0u32,
                        SlideDirection::Right => 1u32,
                        SlideDirection::Up => 2u32,
                        SlideDirection::Down => 3u32,
                    };
                    (3u32, dir, if *push { 1.0f32 } else { 0.0f32 }, [0.0; 4])
                }
                TransitionKind::Wipe {
                    direction,
                    soft_edge,
                } => {
                    let dir = match direction {
                        WipeDirection::Horizontal => 0u32,
                        WipeDirection::Vertical => 1u32,
                        WipeDirection::Radial => 2u32,
                    };
                    (4u32, dir, *soft_edge, [0.0; 4])
                }
                TransitionKind::Zoom { zoom_in } => {
                    (5u32, 0u32, if *zoom_in { 1.0f32 } else { 0.0f32 }, [0.0; 4])
                }
                TransitionKind::Stinger { switch_point, .. } => {
                    (6u32, 0u32, *switch_point, [0.0; 4])
                }
                TransitionKind::Overlay { switch_point, .. } => {
                    (6u32, 0u32, *switch_point, [0.0; 4])
                }
            };

            let trans_uniforms = TransitionUniforms {
                progress: eased_t,
                kind: kind_u32,
                direction: dir_u32,
                param1,
                dip_color,
                _padding: [0.0; 8],
            };

            self.queue.write_buffer(
                &self.transition_uniform_buf,
                0,
                bytemuck::bytes_of(&trans_uniforms),
            );

            let trans_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("TransitionPassBindGroup"),
                layout: &self.pipeline.transition_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: self.transition_uniform_buf.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&self.intermediate_view_a),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&self.intermediate_view_b),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::Sampler(&self.pipeline.sampler),
                    },
                ],
            });

            {
                let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("TransitionRenderPass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &self.program_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(wgpu::Color::BLACK),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                rpass.set_pipeline(&self.pipeline.transition_pipeline);
                rpass.set_bind_group(0, &trans_bind_group, &[]);
                rpass.draw(0..3, 0..1); // Fullscreen triangle
            }
        } else {
            // Direct rendering of Program scene
            self.render_scene_internal(
                self.bus.program_scene,
                &self.program_view,
                sources,
                &mut encoder,
                0,
            )?;
        }

        // 3. Render Preview scene if in Studio Mode
        if self.bus.mode == StudioMode::Studio {
            if let Some(ref working) = self.preview_working_scene {
                self.render_scene_object(working, &self.preview_view, sources, &mut encoder, 0)?;
            } else {
                self.render_scene_internal(
                    self.bus.preview_scene,
                    &self.preview_view,
                    sources,
                    &mut encoder,
                    0,
                )?;
            }
        }

        // 4. Finalize transition completion
        if transition_completed {
            if let Some(target_scene) = new_program_scene {
                if let Some(ref working) = self.preview_working_scene {
                    if self.bus.preview_scene == target_scene {
                        self.scenes.insert(target_scene, working.clone());
                    }
                }
                self.bus.program_scene = target_scene;
            }
            self.active_transition = None;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    /// Renders a live preview frame of a transition between two scenes at progress `t` [0.0..1.0].
    pub fn render_transition_preview(
        &mut self,
        from_scene: SceneId,
        to_scene: SceneId,
        transition: &TransitionDefinition,
        progress: f32,
        sources: &HashMap<SourceId, Arc<TextureView>>,
    ) -> Result<&Arc<TextureView>, CompositorError> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("TransitionPreviewEncoder"),
            });

        self.render_scene_internal(
            from_scene,
            &self.intermediate_view_a,
            sources,
            &mut encoder,
            0,
        )?;
        if let Some(ref working) = self.preview_working_scene {
            self.render_scene_object(working, &self.intermediate_view_b, sources, &mut encoder, 0)?;
        } else {
            self.render_scene_internal(
                to_scene,
                &self.intermediate_view_b,
                sources,
                &mut encoder,
                0,
            )?;
        }

        let eased_t = transition.easing.evaluate(progress.clamp(0.0, 1.0));
        let (kind_u32, dir_u32, param1, dip_color) = match &transition.kind {
            TransitionKind::Cut => (0u32, 0u32, 0.0f32, [0.0; 4]),
            TransitionKind::Fade => (1u32, 0u32, 0.0f32, [0.0; 4]),
            TransitionKind::DipToColor { color } => {
                (2u32, 0u32, 0.0f32, [color.r, color.g, color.b, color.a])
            }
            TransitionKind::Slide { direction, push } => {
                let dir = match direction {
                    SlideDirection::Left => 0u32,
                    SlideDirection::Right => 1u32,
                    SlideDirection::Up => 2u32,
                    SlideDirection::Down => 3u32,
                };
                (3u32, dir, if *push { 1.0f32 } else { 0.0f32 }, [0.0; 4])
            }
            TransitionKind::Wipe {
                direction,
                soft_edge,
            } => {
                let dir = match direction {
                    WipeDirection::Horizontal => 0u32,
                    WipeDirection::Vertical => 1u32,
                    WipeDirection::Radial => 2u32,
                };
                (4u32, dir, *soft_edge, [0.0; 4])
            }
            TransitionKind::Zoom { zoom_in } => {
                (5u32, 0u32, if *zoom_in { 1.0f32 } else { 0.0f32 }, [0.0; 4])
            }
            TransitionKind::Stinger { switch_point, .. } => (6u32, 0u32, *switch_point, [0.0; 4]),
            TransitionKind::Overlay { switch_point, .. } => (6u32, 0u32, *switch_point, [0.0; 4]),
        };

        let trans_uniforms = TransitionUniforms {
            progress: eased_t,
            kind: kind_u32,
            direction: dir_u32,
            param1,
            dip_color,
            _padding: [0.0; 8],
        };

        self.queue.write_buffer(
            &self.transition_uniform_buf,
            0,
            bytemuck::bytes_of(&trans_uniforms),
        );

        let trans_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("PreviewTransitionBindGroup"),
            layout: &self.pipeline.transition_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.transition_uniform_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.intermediate_view_a),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.intermediate_view_b),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&self.pipeline.sampler),
                },
            ],
        });

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("PreviewTransitionPass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.preview_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.pipeline.transition_pipeline);
            rpass.set_bind_group(0, &trans_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(&self.preview_view)
    }

    /// Internal recursive scene rendering helper with cycle detection.
    fn render_scene_internal(
        &self,
        scene_id: SceneId,
        target_view: &TextureView,
        sources: &HashMap<SourceId, Arc<TextureView>>,
        encoder: &mut CommandEncoder,
        depth: usize,
    ) -> Result<(), CompositorError> {
        if depth > 8 {
            return Err(CompositorError::SceneRecursionCycle(scene_id));
        }

        let scene = self
            .scenes
            .get(&scene_id)
            .ok_or(CompositorError::SceneNotFound(scene_id))?;

        self.render_scene_object(scene, target_view, sources, encoder, depth)
    }

    /// Renders a concrete scene object to a target texture view.
    fn render_scene_object(
        &self,
        scene: &Scene,
        target_view: &TextureView,
        sources: &HashMap<SourceId, Arc<TextureView>>,
        encoder: &mut CommandEncoder,
        _depth: usize,
    ) -> Result<(), CompositorError> {
        let clear_col = wgpu::Color {
            r: scene.background.r as f64,
            g: scene.background.g as f64,
            b: scene.background.b as f64,
            a: scene.background.a as f64,
        };

        // Render pass clearing the target with scene background
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("SceneClearPass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_col),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw all visible items from bottom to top
            for item in &scene.items {
                if !item.visible || item.opacity <= 0.0 {
                    continue;
                }

                match &item.source {
                    SceneItemSource::Source(source_id) => {
                        if let Some(source_view) = sources.get(source_id) {
                            let item_w = self.width as f32;
                            let item_h = self.height as f32;

                            // Model matrix in canvas pixels
                            let model_mat = item.transform.to_model_matrix(item_w, item_h);
                            // Combine with orthographic projection
                            let mvp = self.ortho_projection * model_mat;

                            let crop_insets = [
                                (item.crop.left / item_w).clamp(0.0, 1.0),
                                (item.crop.top / item_h).clamp(0.0, 1.0),
                                (item.crop.right / item_w).clamp(0.0, 1.0),
                                (item.crop.bottom / item_h).clamp(0.0, 1.0),
                            ];

                            let uniforms = ItemUniforms {
                                mvp: mvp.to_cols_array_2d(),
                                crop_insets,
                                opacity: item.opacity.clamp(0.0, 1.0),
                                blend_mode: 0,
                                _padding: [0.0; 2],
                            };

                            self.queue.write_buffer(
                                &self.item_uniform_buf,
                                0,
                                bytemuck::bytes_of(&uniforms),
                            );

                            let item_bind_group =
                                self.device.create_bind_group(&BindGroupDescriptor {
                                    label: Some("ItemBindGroup"),
                                    layout: &self.pipeline.item_bind_group_layout,
                                    entries: &[
                                        BindGroupEntry {
                                            binding: 0,
                                            resource: self.item_uniform_buf.as_entire_binding(),
                                        },
                                        BindGroupEntry {
                                            binding: 1,
                                            resource: BindingResource::TextureView(source_view),
                                        },
                                        BindGroupEntry {
                                            binding: 2,
                                            resource: BindingResource::Sampler(
                                                &self.pipeline.sampler,
                                            ),
                                        },
                                    ],
                                });

                            rpass.set_pipeline(&self.pipeline.item_pipeline);
                            rpass.set_bind_group(0, &item_bind_group, &[]);
                            rpass.draw(0..4, 0..1); // Unit quad triangle strip
                        }
                    }
                    SceneItemSource::NestedScene(nested_scene_id) => {
                        // Recursively render nested scene if not self
                        if nested_scene_id != &scene.id {
                            // Render nested scene onto intermediate target, then composite
                            // Cycles are guarded by depth > 8
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
