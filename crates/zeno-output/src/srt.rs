//! SRT (Secure Reliable Transport) streaming configuration, parameter parsing, and URL formatting.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{OutputError, OutputResult};

/// Operating mode for the SRT socket connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SrtMode {
    /// Client initiates connection to remote ingestion server (standard broadcast streaming)
    #[default]
    Caller,
    /// Host listens for incoming SRT broadcast feeds
    Listener,
    /// Bidirectional rendezvous traversal
    Rendezvous,
}

impl SrtMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::Listener => "listener",
            Self::Rendezvous => "rendezvous",
        }
    }
}

/// SRT stream configuration parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrtConfig {
    /// Target host address (IP or domain name)
    pub host: String,
    /// Target port (standard default: 9000)
    pub port: u16,
    /// Connection mode (default: Caller)
    pub mode: SrtMode,
    /// Buffer latency in milliseconds (default: 120 ms for esports low-latency broadcast)
    pub latency_ms: u32,
    /// Optional AES-128 / AES-256 encryption passphrase
    pub passphrase: Option<String>,
    /// Optional stream ID / subscriber identifier (e.g. for Haivision or Nimble streamers)
    pub stream_id: Option<String>,
    /// Connection timeout in milliseconds
    pub connect_timeout_ms: u32,
    /// Max bandwidth overhead percentage (default: 25%)
    pub overhead_bandwidth: u32,
}

impl Default for SrtConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 9000,
            mode: SrtMode::Caller,
            latency_ms: 120,
            passphrase: None,
            stream_id: None,
            connect_timeout_ms: 3000,
            overhead_bandwidth: 25,
        }
    }
}

impl SrtConfig {
    /// Creates a new SRT configuration targeting a host and port with default low latency.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            ..Default::default()
        }
    }

    /// Formats the configuration into a valid `srt://...` endpoint URL accepted by FFmpeg.
    pub fn to_url(&self) -> String {
        let mut query_params = Vec::new();

        query_params.push(format!("mode={}", self.mode.as_str()));
        query_params.push(format!("latency={}", self.latency_ms * 1000)); // SRT URL latency in microseconds
        query_params.push(format!("timeout={}", self.connect_timeout_ms * 1000));
        query_params.push(format!("overheadbandwidth={}", self.overhead_bandwidth));

        if let Some(ref pass) = self.passphrase {
            query_params.push(format!("passphrase={}", urlencoding(pass)));
        }

        if let Some(ref sid) = self.stream_id {
            query_params.push(format!("streamid={}", urlencoding(sid)));
        }

        format!(
            "srt://{}:{}?{}",
            self.host,
            self.port,
            query_params.join("&")
        )
    }

    /// Parses an `srt://...` URL string into an `SrtConfig`.
    pub fn parse_url(url: &str) -> OutputResult<Self> {
        if !url.starts_with("srt://") {
            return Err(OutputError::InvalidParameters(
                "SRT URL must begin with 'srt://'".into(),
            ));
        }

        let without_proto = &url["srt://".len()..];
        let (host_port, query) = match without_proto.split_once('?') {
            Some((hp, q)) => (hp, Some(q)),
            None => (without_proto, None),
        };

        let (host, port) = match host_port.split_once(':') {
            Some((h, p)) => {
                let port_num = p.parse::<u16>().map_err(|_| {
                    OutputError::InvalidParameters(format!("Invalid SRT port number: {}", p))
                })?;
                (h.to_string(), port_num)
            }
            None => (host_port.to_string(), 9000),
        };

        let mut config = Self::new(host, port);

        if let Some(q) = query {
            for pair in q.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    match k.to_lowercase().as_str() {
                        "mode" => match v.to_lowercase().as_str() {
                            "listener" => config.mode = SrtMode::Listener,
                            "rendezvous" => config.mode = SrtMode::Rendezvous,
                            _ => config.mode = SrtMode::Caller,
                        },
                        "latency" => {
                            if let Ok(lat) = v.parse::<u32>() {
                                // Convert microseconds or milliseconds
                                config.latency_ms = if lat > 10000 { lat / 1000 } else { lat };
                            }
                        }
                        "passphrase" => {
                            config.passphrase = Some(urldecoding(v));
                        }
                        "streamid" => {
                            config.stream_id = Some(urldecoding(v));
                        }
                        "timeout" => {
                            if let Ok(to) = v.parse::<u32>() {
                                config.connect_timeout_ms = if to > 10000 { to / 1000 } else { to };
                            }
                        }
                        "overheadbandwidth" => {
                            if let Ok(ob) = v.parse::<u32>() {
                                config.overhead_bandwidth = ob;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(config)
    }

    /// FFmpeg output arguments required for SRT streaming (MPEG-TS container).
    pub fn ffmpeg_output_args(&self) -> Vec<String> {
        vec![
            "-f".into(),
            "mpegts".into(),
            "-muxdelay".into(),
            "0".into(),
            "-pes_payload_size".into(),
            "0".into(),
            self.to_url(),
        ]
    }
}

impl fmt::Display for SrtConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_url())
    }
}

fn urlencoding(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || byte == b'.'
            || byte == b'~'
        {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

fn urldecoding(input: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(b) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                bytes.push(b);
            }
        } else if ch == '+' {
            bytes.push(b' ');
        } else {
            bytes.push(ch as u8);
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srt_config_to_url_and_parse() {
        let mut config = SrtConfig::new("stream.zenocast.live", 9100);
        config.latency_ms = 80;
        config.passphrase = Some("secret_broadcast_key".into());
        config.stream_id = Some("live/main_feed".into());

        let url = config.to_url();
        assert!(url.starts_with("srt://stream.zenocast.live:9100?"));
        assert!(url.contains("latency=80000"));
        assert!(url.contains("passphrase=secret_broadcast_key"));
        assert!(url.contains("streamid=live%2Fmain_feed"));

        let parsed = SrtConfig::parse_url(&url).unwrap();
        assert_eq!(parsed.host, "stream.zenocast.live");
        assert_eq!(parsed.port, 9100);
        assert_eq!(parsed.latency_ms, 80);
        assert_eq!(parsed.passphrase.as_deref(), Some("secret_broadcast_key"));
        assert_eq!(parsed.stream_id.as_deref(), Some("live/main_feed"));
    }

    #[test]
    fn test_srt_ffmpeg_args() {
        let config = SrtConfig::new("192.168.1.100", 9000);
        let args = config.ffmpeg_output_args();
        assert_eq!(args[0], "-f");
        assert_eq!(args[1], "mpegts");
        assert_eq!(args[2], "-muxdelay");
        assert_eq!(args[3], "0");
        assert_eq!(args[4], "-pes_payload_size");
        assert_eq!(args[5], "0");
        assert!(args[6].starts_with("srt://192.168.1.100:9000?"));
    }
}
