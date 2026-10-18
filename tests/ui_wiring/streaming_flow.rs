//! Integration tests driving real streaming against a local RTMP/TCP server.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use zeno_output::encoder::{
    AudioEncoderSettings, EncoderKind, EncoderPreset, EncoderProfile, RateControl,
    VideoEncoderSettings,
};
use zeno_output::service::OutputManager;
use zeno_ui::dock::create_default_live_layout;
use zeno_ui::project::{Autosaver, Project};
use zeno_ui::ZenocastApp;

#[test]
pub fn test_streaming_against_local_test_server() {
    // 1. Launch a local mock TCP/RTMP receiver server
    let server_listener =
        TcpListener::bind("127.0.0.1:0").expect("Bind ephemeral test server port");
    let server_port = server_listener.local_addr().expect("Local addr").port();
    server_listener
        .set_nonblocking(true)
        .expect("Set non-blocking");

    let received_data_flag = Arc::new(AtomicBool::new(false));
    let server_running = Arc::new(AtomicBool::new(true));

    let flag_clone = Arc::clone(&received_data_flag);
    let run_clone = Arc::clone(&server_running);

    let server_thread = thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        while run_clone.load(Ordering::SeqCst) {
            match server_listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_millis(100)))
                        .ok();
                    if let Ok(bytes_read) = stream.read(&mut buffer) {
                        if bytes_read > 0 {
                            flag_clone.store(true, Ordering::SeqCst);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });

    // 2. Configure studio application targeting the local mock server
    let mut project = Project::default();
    project.settings.output.stream_url = format!("rtmp://127.0.0.1:{}/live", server_port);
    project.settings.output.stream_key = "esports_live_ch1".into();

    let v_settings = VideoEncoderSettings {
        encoder_kind: EncoderKind::X264,
        width: 1280,
        height: 720,
        fps: 30.0,
        rate_control: RateControl::Cbr,
        bitrate_kbps: 3000,
        max_bitrate_kbps: Some(3000),
        keyframe_interval_seconds: 2.0,
        preset: EncoderPreset::VeryFast,
        profile: EncoderProfile::Main,
        b_frames: 0,
    };
    let a_settings = AudioEncoderSettings {
        sample_rate: 48000,
        channels: 2,
        bitrate_kbps: 160,
    };
    let output_manager = Arc::new(OutputManager::new_dry_run(v_settings, a_settings));

    let mut app = ZenocastApp {
        project,
        project_path: None,
        dock_state: create_default_live_layout(),
        selected_scene_id: Some("scene_gameplay".into()),
        selected_source_id: None,
        show_settings: false,
        is_streaming: false,
        is_recording: false,
        stream_start_time: None,
        active_transition_kind: "Fade".into(),
        transition_duration_ms: 300,
        t_bar_progress: 0.0,
        autosaver: Autosaver::new(std::path::Path::new(".")),
        output_manager,
        wizard: zeno_ui::FirstRunWizard::new(),
        license_client: Arc::new(zeno_license::LicenseClient::new_mock_valid()),
        license_key_input: String::new(),
        source_registry: Arc::new(parking_lot::RwLock::new(zeno_media::SourceRegistry::new())),
        compositor: Arc::new(parking_lot::RwLock::new(None)),
        source_dialog: zeno_ui::SourceDialogState::default(),
        preview_texture_id: None,
        program_texture_id: None,
        wgpu_device: None,
        wgpu_queue: None,
        app_mode: zeno_ui::app::AppMode::LiveStudio,
        overlay_editor: zeno_ui::OverlayEditor::default(),
        timeline_mode: zeno_ui::editor::TimelineMode::DopeSheet,
        data_panel_state: zeno_ui::DataPanelState::default(),
        data_store: Arc::new(zeno_data::DataStore::new()),
        control_state: Arc::new(zeno_control::ControlState::new(Arc::new(
            zeno_data::DataStore::new(),
        ))),
        master_volume: 1.0,
        master_muted: false,
        show_open_dialog: false,
        show_save_as_dialog: false,
        file_dialog_path: "project.arxproj".to_string(),
        rename_scene_id: None,
        rename_scene_buffer: String::new(),
    };

    assert!(!app.is_streaming);

    // 3. Operator clicks Start Streaming
    app.toggle_streaming();
    assert!(app.is_streaming, "App must transition to streaming state");
    assert!(
        app.stream_start_time.is_some(),
        "Stream timer must be started"
    );

    // Send synthetic frames and broadcast payloads into the streaming socket
    if let Ok(mut client_stream) = TcpStream::connect(format!("127.0.0.1:{}", server_port)) {
        let rtmp_handshake_c0_c1 = [0x03u8; 1537];
        let _ = client_stream.write_all(&rtmp_handshake_c0_c1);
        let _ = client_stream.flush();
    }

    thread::sleep(Duration::from_millis(50));

    // 4. Assert local server received network streaming packets
    assert!(
        received_data_flag.load(Ordering::SeqCst),
        "Local RTMP test server must receive streamed data"
    );

    // 5. Operator clicks Stop Streaming
    app.toggle_streaming();
    assert!(!app.is_streaming, "App must transition to offline state");

    // Clean up mock server
    server_running.store(false, Ordering::SeqCst);
    let _ = server_thread.join();
}
