//! Modal type picker and properties configuration dialog for sources.
//!
//! Strictly adheres to Section 7 & 12 broadcast design system:
//! flat surfaces, pure flat styling, zero color literals outside `theme.rs`.

use egui::{vec2, Align2, Window};

use crate::project::SourceKind;
use crate::theme::{AMBER_WARN, GRID_UNIT, RED_LIVE, TEXT, TEXT_MUTED};
use zeno_media::DeviceManager;

/// Operating mode of the source dialog modal.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SourceDialogMode {
    #[default]
    Closed,
    /// Type picker menu allowing user to choose what kind of source to create.
    TypePicker,
    /// Properties panel configuring the chosen source type.
    Properties {
        kind: SourceKind,
        edit_source_id: Option<String>,
    },
}

/// Action emitted when user confirms creation or modification of a source.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceDialogAction {
    Create {
        name: String,
        kind: SourceKind,
        target: String,
        width: u32,
        height: u32,
        color_rgba: [f32; 4],
        capture_cursor: bool,
        file_path: String,
        text_content: String,
        font_size: f32,
        url: String,
        looping: bool,
        volume: f32,
        muted: bool,
    },
    Update {
        id: String,
        name: String,
        target: String,
        width: u32,
        height: u32,
        color_rgba: [f32; 4],
        capture_cursor: bool,
        file_path: String,
        text_content: String,
        font_size: f32,
        url: String,
        looping: bool,
        volume: f32,
        muted: bool,
    },
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn create_hidden_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Generates or extracts a thumbnail (RGBA8) for an image or video file.
pub fn load_media_thumbnail(file_path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return None;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => {
            if let Ok(dyn_img) = image::open(path) {
                let thumb = dyn_img.thumbnail(256, 144);
                let rgba = thumb.to_rgba8();
                return Some((rgba.width(), rgba.height(), rgba.into_raw()));
            }
        }
        "mp4" | "mov" | "mkv" | "webm" => {
            let mut cmd = create_hidden_command("ffmpeg");
            if let Ok(out) = cmd
                .args([
                    "-nostdin",
                    "-y",
                    "-ss",
                    "00:00:00",
                    "-i",
                    file_path,
                    "-vframes",
                    "1",
                    "-f",
                    "image2pipe",
                    "-vcodec",
                    "png",
                    "-",
                ])
                .output()
            {
                if out.status.success() && !out.stdout.is_empty() {
                    if let Ok(dyn_img) = image::load_from_memory(&out.stdout) {
                        let thumb = dyn_img.thumbnail(256, 144);
                        let rgba = thumb.to_rgba8();
                        return Some((rgba.width(), rgba.height(), rgba.into_raw()));
                    }
                }
            }
            // Clean fallback thumbnail for video if ffmpeg not available:
            let w = 256u32;
            let h = 144u32;
            let mut buf = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in 0..w {
                    let idx = ((y * w + x) * 4) as usize;
                    let is_border = x < 3 || x >= w - 3 || y < 3 || y >= h - 3;
                    if is_border {
                        buf[idx] = 60;
                        buf[idx + 1] = 65;
                        buf[idx + 2] = 80;
                        buf[idx + 3] = 255;
                    } else {
                        buf[idx] = 25;
                        buf[idx + 1] = 28;
                        buf[idx + 2] = 35;
                        buf[idx + 3] = 255;
                    }
                }
            }
            return Some((w, h, buf));
        }
        _ => {}
    }
    None
}

/// In-memory state tracking source type picking and properties editing.
#[derive(Clone)]
pub struct SourceDialogState {
    pub mode: SourceDialogMode,
    pub name: String,
    pub color_rgba: [f32; 4],
    pub display_index: usize,
    pub window_title: String,
    pub camera_device: String,
    pub audio_device: String,
    pub scale_mode: String,
    pub file_path: String,
    pub media_files: Vec<String>,
    pub slideshow_interval_sec: f32,
    pub is_playing: bool,
    pub text_content: String,
    pub font_size: f32,
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub capture_cursor: bool,
    pub looping: bool,
    pub volume: f32,
    pub muted: bool,
    pub error_message: Option<String>,
    pub thumbnail_rgba: Option<(u32, u32, Vec<u8>)>,
    pub thumbnail_tex: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for SourceDialogState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceDialogState")
            .field("mode", &self.mode)
            .field("name", &self.name)
            .field("color_rgba", &self.color_rgba)
            .field("display_index", &self.display_index)
            .field("window_title", &self.window_title)
            .field("camera_device", &self.camera_device)
            .field("audio_device", &self.audio_device)
            .field("scale_mode", &self.scale_mode)
            .field("file_path", &self.file_path)
            .field("media_files", &self.media_files)
            .field("slideshow_interval_sec", &self.slideshow_interval_sec)
            .field("is_playing", &self.is_playing)
            .field("text_content", &self.text_content)
            .field("font_size", &self.font_size)
            .field("url", &self.url)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("capture_cursor", &self.capture_cursor)
            .field("looping", &self.looping)
            .field("volume", &self.volume)
            .field("muted", &self.muted)
            .field("error_message", &self.error_message)
            .field("has_thumbnail", &self.thumbnail_rgba.is_some())
            .finish()
    }
}

impl PartialEq for SourceDialogState {
    fn eq(&self, other: &Self) -> bool {
        self.mode == other.mode
            && self.name == other.name
            && self.color_rgba == other.color_rgba
            && self.display_index == other.display_index
            && self.window_title == other.window_title
            && self.camera_device == other.camera_device
            && self.audio_device == other.audio_device
            && self.scale_mode == other.scale_mode
            && self.file_path == other.file_path
            && self.media_files == other.media_files
            && self.slideshow_interval_sec == other.slideshow_interval_sec
            && self.is_playing == other.is_playing
            && self.text_content == other.text_content
            && self.font_size == other.font_size
            && self.url == other.url
            && self.width == other.width
            && self.height == other.height
            && self.capture_cursor == other.capture_cursor
            && self.looping == other.looping
            && self.volume == other.volume
            && self.muted == other.muted
            && self.error_message == other.error_message
            && self.thumbnail_rgba == other.thumbnail_rgba
    }
}

impl Default for SourceDialogState {
    fn default() -> Self {
        Self {
            mode: SourceDialogMode::Closed,
            name: String::new(),
            color_rgba: [0.10, 0.40, 0.90, 1.0],
            display_index: 0,
            window_title: String::new(),
            camera_device: String::new(),
            audio_device: String::new(),
            scale_mode: "Fit".to_string(),
            file_path: String::new(),
            media_files: Vec::new(),
            slideshow_interval_sec: 5.0,
            is_playing: true,
            text_content: "Zenocast Live".into(),
            font_size: 48.0,
            url: "https://zenocast.live".into(),
            width: 1920,
            height: 1080,
            capture_cursor: true,
            looping: true,
            volume: 1.0,
            muted: false,
            error_message: None,
            thumbnail_rgba: None,
            thumbnail_tex: None,
        }
    }
}

impl SourceDialogState {
    /// Opens the modal in TypePicker mode to add a new source.
    pub fn open_add(&mut self) {
        *self = Self::default();
        self.mode = SourceDialogMode::TypePicker;
    }

    /// Opens the modal in Properties mode pre-filled with a media file path (e.g. from drag-and-drop).
    pub fn open_add_media_with_path(&mut self, path: String) {
        *self = Self::default();
        let stem = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Media Source")
            .to_string();
        self.name = stem;
        self.file_path = path.clone();
        self.media_files = vec![path];
        self.mode = SourceDialogMode::Properties {
            kind: SourceKind::MediaFile,
            edit_source_id: None,
        };
    }

    /// Ensures thumbnail texture is initialized for preview rendering.
    pub fn ensure_thumbnail(&mut self, ctx: &egui::Context) {
        if self.thumbnail_tex.is_none() {
            if let Some((w, h, ref rgba)) = self.thumbnail_rgba {
                let color_img =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba);
                self.thumbnail_tex =
                    Some(ctx.load_texture("media_thumb", color_img, egui::TextureOptions::LINEAR));
            } else if !self.file_path.trim().is_empty()
                && std::path::Path::new(&self.file_path).exists()
            {
                if let Some((w, h, rgba)) = load_media_thumbnail(&self.file_path) {
                    let color_img =
                        egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                    self.thumbnail_tex = Some(ctx.load_texture(
                        "media_thumb",
                        color_img,
                        egui::TextureOptions::LINEAR,
                    ));
                    self.thumbnail_rgba = Some((w, h, rgba));
                }
            }
        }
    }

    /// Opens the modal in Properties mode pre-filled with an existing source's settings.
    #[allow(clippy::too_many_arguments)]
    pub fn open_edit(
        &mut self,
        id: &str,
        name: &str,
        kind: SourceKind,
        target: &str,
        width: f32,
        height: f32,
        volume: f32,
        muted: bool,
    ) {
        *self = Self::default();
        self.name = name.to_string();
        self.width = width as u32;
        self.height = height as u32;
        self.volume = volume;
        self.muted = muted;

        match kind {
            SourceKind::Color => {
                self.color_rgba = [0.10, 0.40, 0.90, 1.0];
            }
            SourceKind::DisplayCapture => {
                self.display_index = 0;
            }
            SourceKind::WindowCapture => {
                self.window_title = target.to_string();
            }
            SourceKind::CameraFeed => {
                self.camera_device = target.to_string();
            }
            SourceKind::Image | SourceKind::MediaFile => {
                self.file_path = target.to_string();
                self.media_files = vec![target.to_string()];
            }
            SourceKind::Text => {
                self.text_content = target.to_string();
            }
            SourceKind::Browser => {
                self.url = target.to_string();
            }
            SourceKind::AudioInput | SourceKind::AudioOutput => {
                self.audio_device = target.to_string();
            }
            _ => {}
        }

        self.mode = SourceDialogMode::Properties {
            kind,
            edit_source_id: Some(id.to_string()),
        };
    }

    /// Closes the modal.
    pub fn close(&mut self) {
        self.mode = SourceDialogMode::Closed;
        self.error_message = None;
    }

    /// Renders the source modal (either type picker or properties dialog).
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<SourceDialogAction> {
        let mut action = None;

        match self.mode.clone() {
            SourceDialogMode::Closed => {}
            SourceDialogMode::TypePicker => {
                let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                if escape_pressed {
                    self.close();
                }

                Window::new("Add Source - Select Type")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .fixed_size(vec2(480.0, 420.0))
                    .show(ctx, |ui| {
                        ui.spacing_mut().item_spacing = vec2(GRID_UNIT, GRID_UNIT);
                        ui.colored_label(
                            TEXT_MUTED,
                            "Choose the kind of visual or audio source to add:",
                        );
                        ui.add_space(GRID_UNIT);

                        let types = [
                            (
                                "Color Source",
                                SourceKind::Color,
                                "Solid flat color fill for background or stage bounds",
                            ),
                            (
                                "Display Capture",
                                SourceKind::DisplayCapture,
                                "Primary or secondary physical desktop monitor",
                            ),
                            (
                                "Window Capture",
                                SourceKind::WindowCapture,
                                "Specific application or game window",
                            ),
                            (
                                "Camera / Capture Card",
                                SourceKind::CameraFeed,
                                "USB UVC webcam, PCIe HDMI or SDI capture feed",
                            ),
                            (
                                "Media Source",
                                SourceKind::MediaFile,
                                "Video (MP4, MKV, MOV), image (PNG, JPG), or GIF with auto-detected playback",
                            ),
                            (
                                "Text Source",
                                SourceKind::Text,
                                "Formatted text element or reactive data binding",
                            ),
                            (
                                "Audio Input (Microphone)",
                                SourceKind::AudioInput,
                                "Hardware microphone or line-in audio input",
                            ),
                            (
                                "Audio Output (Desktop)",
                                SourceKind::AudioOutput,
                                "System or application audio loopback capture",
                            ),
                            (
                                "Browser Source",
                                SourceKind::Browser,
                                "Web page or HTML5 animated tournament overlay",
                            ),
                        ];

                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                for (title, kind, desc) in types {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            if ui.button(format!("+ {}", title)).clicked() {
                                                self.name = title.to_string();
                                                self.mode = SourceDialogMode::Properties {
                                                    kind,
                                                    edit_source_id: None,
                                                };
                                            }
                                            ui.colored_label(TEXT_MUTED, desc);
                                        });
                                    });
                                }
                            });

                        ui.add_space(GRID_UNIT);
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.close();
                            }
                        });
                    });
            }
            SourceDialogMode::Properties {
                kind,
                edit_source_id,
            } => {
                let is_edit = edit_source_id.is_some();
                let title = if is_edit {
                    format!("Source Properties: {}", self.name)
                } else {
                    match kind {
                        SourceKind::MediaFile | SourceKind::Image => {
                            "New Source: Media Source".to_string()
                        }
                        _ => format!("New Source: {:?}", kind),
                    }
                };

                let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                if escape_pressed {
                    self.close();
                }

                Window::new(title)
                    .collapsible(false)
                    .resizable(true)
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .default_size(vec2(540.0, 520.0))
                    .show(ctx, |ui| {
                        ui.spacing_mut().item_spacing = vec2(GRID_UNIT, GRID_UNIT);

                        // Error notification banner if any validation fails
                        if let Some(ref err) = self.error_message {
                            ui.colored_label(RED_LIVE, format!("⚠ {}", err));
                            ui.add_space(4.0);
                        }

                        ui.horizontal(|ui| {
                            ui.colored_label(TEXT, "Source Name:");
                            ui.text_edit_singleline(&mut self.name);
                        });
                        ui.separator();

                        match kind {
                            SourceKind::Color => {
                                ui.colored_label(TEXT, "Fill Color (RGBA):");
                                ui.horizontal(|ui| {
                                    let mut color_u8 = [
                                        (self.color_rgba[0] * 255.0) as u8,
                                        (self.color_rgba[1] * 255.0) as u8,
                                        (self.color_rgba[2] * 255.0) as u8,
                                        (self.color_rgba[3] * 255.0) as u8,
                                    ];
                                    if ui
                                        .color_edit_button_srgba_unmultiplied(&mut color_u8)
                                        .changed()
                                    {
                                        self.color_rgba = [
                                            color_u8[0] as f32 / 255.0,
                                            color_u8[1] as f32 / 255.0,
                                            color_u8[2] as f32 / 255.0,
                                            color_u8[3] as f32 / 255.0,
                                        ];
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Resolution:");
                                    ui.add(egui::DragValue::new(&mut self.width).prefix("Width: "));
                                    ui.add(
                                        egui::DragValue::new(&mut self.height).prefix("Height: "),
                                    );
                                });
                            }
                            SourceKind::DisplayCapture => {
                                let dev_mgr = DeviceManager::new();
                                let displays = dev_mgr.list_displays();

                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Monitor:");
                                    let current_name = displays
                                        .get(self.display_index)
                                        .map(|d| {
                                            format!("{} ({}x{})", d.name, d.bounds.2, d.bounds.3)
                                        })
                                        .unwrap_or_else(|| "Primary Display".into());

                                    egui::ComboBox::from_id_salt("disp_picker")
                                        .selected_text(current_name)
                                        .show_ui(ui, |ui| {
                                            for (idx, disp) in displays.iter().enumerate() {
                                                let label = format!(
                                                    "{} ({}x{})",
                                                    disp.name, disp.bounds.2, disp.bounds.3
                                                );
                                                if ui
                                                    .selectable_label(
                                                        self.display_index == idx,
                                                        label,
                                                    )
                                                    .clicked()
                                                {
                                                    self.display_index = idx;
                                                    self.width = disp.bounds.2;
                                                    self.height = disp.bounds.3;
                                                }
                                            }
                                        });
                                });

                                ui.checkbox(&mut self.capture_cursor, "Capture Mouse Cursor");
                            }
                            SourceKind::WindowCapture => {
                                let dev_mgr = DeviceManager::new();
                                let windows = dev_mgr.list_windows();

                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Window:");
                                    let current = if self.window_title.is_empty() {
                                        "Select Window...".to_string()
                                    } else {
                                        self.window_title.clone()
                                    };

                                    egui::ComboBox::from_id_salt("win_picker")
                                        .selected_text(current)
                                        .show_ui(ui, |ui| {
                                            for win in &windows {
                                                if ui
                                                    .selectable_label(
                                                        self.window_title == win.title,
                                                        &win.title,
                                                    )
                                                    .clicked()
                                                {
                                                    self.window_title = win.title.clone();
                                                }
                                            }
                                        });
                                });

                                ui.checkbox(&mut self.capture_cursor, "Capture Mouse Cursor");
                            }
                            SourceKind::CameraFeed => {
                                let dev_mgr = DeviceManager::new();
                                let cameras = dev_mgr.list_cameras();

                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Camera Device:");
                                    let current = if self.camera_device.is_empty() {
                                        cameras
                                            .first()
                                            .map(|c| c.name.clone())
                                            .unwrap_or_else(|| "Virtual Broadcast Camera (Simulation)".into())
                                    } else {
                                        self.camera_device.clone()
                                    };

                                    egui::ComboBox::from_id_salt("cam_picker")
                                        .selected_text(current)
                                        .show_ui(ui, |ui| {
                                            for cam in &cameras {
                                                if ui
                                                    .selectable_label(
                                                        self.camera_device == cam.name,
                                                        &cam.name,
                                                    )
                                                    .clicked()
                                                {
                                                    self.camera_device = cam.name.clone();
                                                    if let Some(fmt) = cam.formats.first() {
                                                        self.width = fmt.width;
                                                        self.height = fmt.height;
                                                    }
                                                }
                                            }
                                        });
                                });

                                if cameras.is_empty() {
                                    ui.colored_label(
                                        AMBER_WARN,
                                        "No physical cameras detected. Virtual broadcast simulation will be used.",
                                    );
                                } else if let Some(selected_cam) =
                                    cameras.iter().find(|c| c.name == self.camera_device)
                                {
                                    if !selected_cam.formats.is_empty() {
                                        ui.horizontal(|ui| {
                                            ui.colored_label(TEXT, "Resolution / Format:");
                                            let cur_fmt = format!(
                                                "{}x{} ({})",
                                                self.width, self.height, selected_cam.formats[0].format_name
                                            );
                                            egui::ComboBox::from_id_salt("cam_fmt_picker")
                                                .selected_text(cur_fmt)
                                                .show_ui(ui, |ui| {
                                                    for fmt in &selected_cam.formats {
                                                        let label = format!(
                                                            "{}x{} @ {:.0}fps ({})",
                                                            fmt.width,
                                                            fmt.height,
                                                            fmt.frame_rate,
                                                            fmt.format_name
                                                        );
                                                        if ui
                                                            .selectable_label(
                                                                self.width == fmt.width
                                                                    && self.height == fmt.height,
                                                                label,
                                                            )
                                                            .clicked()
                                                        {
                                                            self.width = fmt.width;
                                                            self.height = fmt.height;
                                                        }
                                                    }
                                                });
                                        });
                                    }
                                }
                            }
                            SourceKind::Image | SourceKind::MediaFile => {
                                // Drag & drop detection onto dialog
                                ctx.input(|i| {
                                    if let Some(file) = i.raw.dropped_files.first() {
                                        if let Some(ref path) = file.path {
                                            let p = path.to_string_lossy().to_string();
                                            self.file_path = p.clone();
                                            self.media_files = vec![p];
                                            self.thumbnail_tex = None;
                                            self.thumbnail_rgba = None;
                                        }
                                    }
                                });

                                self.ensure_thumbnail(ctx);

                                // File path picker row
                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Media File:");
                                    let display_path = if self.file_path.is_empty() {
                                        "No file selected".to_string()
                                    } else {
                                        self.file_path.clone()
                                    };
                                    ui.add(egui::Label::new(egui::RichText::new(&display_path).monospace()).truncate());
                                });

                                ui.horizontal(|ui| {
                                    if ui.button("Browse…").clicked() {
                                        let dialog = rfd::FileDialog::new()
                                            .add_filter("Media Files", &["mp4", "mov", "mkv", "webm", "png", "jpg", "jpeg", "gif", "webp"])
                                            .add_filter("Video Files", &["mp4", "mov", "mkv", "webm"])
                                            .add_filter("Image Files", &["png", "jpg", "jpeg", "gif", "webp"])
                                            .add_filter("All Files", &["*"]);
                                        if let Some(path) = dialog.pick_file() {
                                            let p = path.to_string_lossy().to_string();
                                            self.file_path = p.clone();
                                            self.media_files = vec![p];
                                            self.thumbnail_tex = None;
                                            self.thumbnail_rgba = None;
                                            self.ensure_thumbnail(ctx);
                                        }
                                    }

                                    if ui.button("Browse Multiple…").on_hover_text("Select multiple images for slideshow").clicked() {
                                        let dialog = rfd::FileDialog::new()
                                            .add_filter("Image Files", &["png", "jpg", "jpeg", "gif", "webp"])
                                            .add_filter("All Files", &["*"]);
                                        if let Some(paths) = dialog.pick_files() {
                                            if !paths.is_empty() {
                                                self.media_files = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
                                                self.file_path = self.media_files[0].clone();
                                                self.thumbnail_tex = None;
                                                self.thumbnail_rgba = None;
                                                self.ensure_thumbnail(ctx);
                                            }
                                        }
                                    }

                                    if !self.file_path.is_empty()
                                        && ui.button("❌").on_hover_text("Clear selected file").clicked()
                                    {
                                        self.file_path.clear();
                                        self.media_files.clear();
                                        self.thumbnail_tex = None;
                                        self.thumbnail_rgba = None;
                                    }
                                });

                                // Thumbnail preview
                                if let Some(ref tex) = self.thumbnail_tex {
                                    ui.add_space(4.0);
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            let (rect, _resp) = ui.allocate_exact_size(vec2(160.0, 90.0), egui::Sense::hover());
                                            ui.painter().image(
                                                tex.id(),
                                                rect,
                                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                                egui::Color32::WHITE,
                                            );
                                            ui.vertical(|ui| {
                                                if let Some(fname) = std::path::Path::new(&self.file_path).file_name().and_then(|f| f.to_str()) {
                                                    ui.colored_label(TEXT, fname);
                                                }
                                                if self.media_files.len() > 1 {
                                                    ui.colored_label(TEXT_MUTED, format!("Slideshow: {} files", self.media_files.len()));
                                                }
                                                let ext_upper = std::path::Path::new(&self.file_path)
                                                    .extension()
                                                    .and_then(|e| e.to_str())
                                                    .map(|e| e.to_ascii_uppercase())
                                                    .unwrap_or_default();
                                                ui.colored_label(TEXT_MUTED, format!("Type: {}", ext_upper));
                                            });
                                        });
                                    });
                                    ui.add_space(4.0);
                                }

                                // Auto-detect media behavior
                                let ext = std::path::Path::new(&self.file_path)
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .map(|e| e.to_ascii_lowercase())
                                    .unwrap_or_default();
                                let is_video = matches!(ext.as_str(), "mp4" | "mov" | "mkv" | "webm");
                                let is_gif = ext == "gif";

                                if is_video || is_gif {
                                    ui.group(|ui| {
                                        ui.colored_label(TEXT, "Transport & Playback:");
                                        ui.horizontal(|ui| {
                                            let play_text = if self.is_playing { "⏸ Pause" } else { "▶ Play" };
                                            if ui.button(play_text).clicked() {
                                                self.is_playing = !self.is_playing;
                                            }
                                            if ui.button("⏹ Stop / Rewind").clicked() {
                                                self.is_playing = false;
                                            }
                                        });
                                        ui.checkbox(&mut self.looping, "Loop Playback Continuously");
                                        if is_video {
                                            ui.horizontal(|ui| {
                                                ui.colored_label(TEXT, "Volume:");
                                                ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.5).text("Gain"));
                                            });
                                        }
                                    });
                                }

                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Scale Mode:");
                                    egui::ComboBox::from_id_salt("media_scale_mode")
                                        .selected_text(&self.scale_mode)
                                        .show_ui(ui, |ui| {
                                            for mode in &["Fit", "Fill", "Stretch", "Center"] {
                                                if ui.selectable_label(self.scale_mode == *mode, *mode).clicked() {
                                                    self.scale_mode = (*mode).to_string();
                                                }
                                            }
                                        });
                                });

                                if self.media_files.len() > 1 {
                                    ui.horizontal(|ui| {
                                        ui.colored_label(TEXT, "Slideshow Interval:");
                                        ui.add(egui::Slider::new(&mut self.slideshow_interval_sec, 1.0..=60.0).text("sec"));
                                    });
                                }

                                if !self.file_path.trim().is_empty()
                                    && !std::path::Path::new(&self.file_path).exists()
                                {
                                    ui.colored_label(
                                        AMBER_WARN,
                                        "Notice: Media file does not exist on disk (test placeholder will be rendered)",
                                    );
                                }
                            }
                            SourceKind::Text => {
                                ui.colored_label(TEXT, "Text Content:");
                                ui.text_edit_multiline(&mut self.text_content);
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Slider::new(&mut self.font_size, 12.0..=120.0)
                                            .text("Font Size"),
                                    );
                                });
                                let mut color_u8 = [
                                    (self.color_rgba[0] * 255.0) as u8,
                                    (self.color_rgba[1] * 255.0) as u8,
                                    (self.color_rgba[2] * 255.0) as u8,
                                    (self.color_rgba[3] * 255.0) as u8,
                                ];
                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Text Color:");
                                    if ui
                                        .color_edit_button_srgba_unmultiplied(&mut color_u8)
                                        .changed()
                                    {
                                        self.color_rgba = [
                                            color_u8[0] as f32 / 255.0,
                                            color_u8[1] as f32 / 255.0,
                                            color_u8[2] as f32 / 255.0,
                                            color_u8[3] as f32 / 255.0,
                                        ];
                                    }
                                });
                            }
                            SourceKind::AudioInput => {
                                let (inputs, _) = zeno_audio::enumerate_audio_devices();
                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Input Device:");
                                    let current = if self.audio_device.is_empty() {
                                        inputs
                                            .first()
                                            .map(|d| d.name.clone())
                                            .unwrap_or_else(|| "Default Microphone (WASAPI)".into())
                                    } else {
                                        self.audio_device.clone()
                                    };

                                    egui::ComboBox::from_id_salt("audio_in_picker")
                                        .selected_text(current)
                                        .show_ui(ui, |ui| {
                                            for dev in &inputs {
                                                if ui
                                                    .selectable_label(
                                                        self.audio_device == dev.name,
                                                        &dev.name,
                                                    )
                                                    .clicked()
                                                {
                                                    self.audio_device = dev.name.clone();
                                                }
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Gain"),
                                    );
                                    ui.checkbox(&mut self.muted, "Mute");
                                });
                            }
                            SourceKind::AudioOutput => {
                                let (_, outputs) = zeno_audio::enumerate_audio_devices();
                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Loopback Device:");
                                    let current = if self.audio_device.is_empty() {
                                        outputs
                                            .first()
                                            .map(|d| d.name.clone())
                                            .unwrap_or_else(|| "Default Output (WASAPI)".into())
                                    } else {
                                        self.audio_device.clone()
                                    };

                                    egui::ComboBox::from_id_salt("audio_out_picker")
                                        .selected_text(current)
                                        .show_ui(ui, |ui| {
                                            for dev in &outputs {
                                                if ui
                                                    .selectable_label(
                                                        self.audio_device == dev.name,
                                                        &dev.name,
                                                    )
                                                    .clicked()
                                                {
                                                    self.audio_device = dev.name.clone();
                                                }
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Gain"),
                                    );
                                    ui.checkbox(&mut self.muted, "Mute");
                                });
                            }
                            SourceKind::Browser => {
                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "URL:");
                                    ui.text_edit_singleline(&mut self.url);
                                });
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut self.width).prefix("Width: "));
                                    ui.add(
                                        egui::DragValue::new(&mut self.height).prefix("Height: "),
                                    );
                                });
                            }
                            SourceKind::ArxOverlay => {
                                ui.horizontal(|ui| {
                                    ui.colored_label(TEXT, "Package (.arx):");
                                    ui.text_edit_singleline(&mut self.file_path);
                                    if ui.button("Browse…").clicked() {
                                        let dialog = rfd::FileDialog::new()
                                            .add_filter("Zenocast Overlay (.arx)", &["arx"])
                                            .add_filter("All Files", &["*"]);
                                        if let Some(path) = dialog.pick_file() {
                                            self.file_path = path.to_string_lossy().to_string();
                                        }
                                    }
                                });
                            }
                        }

                        ui.add_space(GRID_UNIT * 2.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            let confirm_label = if is_edit {
                                "Apply Changes"
                            } else {
                                "Add Source"
                            };
                            let confirm_clicked = ui.button(confirm_label).clicked()
                                || (enter_pressed && kind != SourceKind::Text);
                            if confirm_clicked {
                                // Input Validation
                                if self.name.trim().is_empty() {
                                    self.error_message = Some("Source name cannot be empty".into());
                                    return;
                                }

                                match kind {
                                    SourceKind::Browser if self.url.trim().is_empty() => {
                                        self.error_message = Some("URL cannot be empty".into());
                                        return;
                                    }
                                    SourceKind::WindowCapture
                                        if self.window_title.trim().is_empty() =>
                                    {
                                        self.error_message =
                                            Some("Please select an application window".into());
                                        return;
                                    }
                                    _ => {}
                                }

                                let target = match kind {
                                    SourceKind::Color => format!(
                                        "rgba({:.2},{:.2},{:.2},{:.2})",
                                        self.color_rgba[0],
                                        self.color_rgba[1],
                                        self.color_rgba[2],
                                        self.color_rgba[3]
                                    ),
                                    SourceKind::DisplayCapture => {
                                        format!("Display {}", self.display_index)
                                    }
                                    SourceKind::WindowCapture => self.window_title.clone(),
                                    SourceKind::CameraFeed => self.camera_device.clone(),
                                    SourceKind::Image
                                    | SourceKind::MediaFile
                                    | SourceKind::ArxOverlay => self.file_path.clone(),
                                    SourceKind::Text => self.text_content.clone(),
                                    SourceKind::AudioInput | SourceKind::AudioOutput => {
                                        if self.audio_device.is_empty() {
                                            "Default Audio Endpoint".into()
                                        } else {
                                            self.audio_device.clone()
                                        }
                                    }
                                    SourceKind::Browser => self.url.clone(),
                                };

                                if let Some(ref id) = edit_source_id {
                                    action = Some(SourceDialogAction::Update {
                                        id: id.clone(),
                                        name: self.name.clone(),
                                        target,
                                        width: self.width,
                                        height: self.height,
                                        color_rgba: self.color_rgba,
                                        capture_cursor: self.capture_cursor,
                                        file_path: self.file_path.clone(),
                                        text_content: self.text_content.clone(),
                                        font_size: self.font_size,
                                        url: self.url.clone(),
                                        looping: self.looping,
                                        volume: self.volume,
                                        muted: self.muted,
                                    });
                                } else {
                                    action = Some(SourceDialogAction::Create {
                                        name: self.name.clone(),
                                        kind,
                                        target,
                                        width: self.width,
                                        height: self.height,
                                        color_rgba: self.color_rgba,
                                        capture_cursor: self.capture_cursor,
                                        file_path: self.file_path.clone(),
                                        text_content: self.text_content.clone(),
                                        font_size: self.font_size,
                                        url: self.url.clone(),
                                        looping: self.looping,
                                        volume: self.volume,
                                        muted: self.muted,
                                    });
                                }

                                self.close();
                            }

                            if ui.button("Cancel").clicked() {
                                self.close();
                            }
                        });
                    });
            }
        }

        action
    }
}
