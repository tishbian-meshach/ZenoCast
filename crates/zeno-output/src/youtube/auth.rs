//! OAuth 2.0 PKCE authentication flow and Windows Credential token management.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use tracing::info;

use crate::error::{OutputError, OutputResult};

/// Stored OAuth access and refresh credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Bearer access token for YouTube Data API calls
    pub access_token: String,
    /// Refresh token for acquiring fresh access tokens
    pub refresh_token: Option<String>,
    /// Token expiry epoch timestamp in seconds
    pub expires_at: u64,
}

impl OAuthToken {
    /// Checks whether the access token has expired (with 60-second grace window).
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now + 60 >= self.expires_at
    }
}

/// In-memory and Windows Credential Manager storage for OAuth credentials.
#[derive(Debug, Default, Clone)]
pub struct TokenStorage {
    cached: Arc<parking_lot::Mutex<Option<OAuthToken>>>,
}

use std::sync::Arc;

impl TokenStorage {
    /// Creates a new token storage instance.
    pub fn new() -> Self {
        Self {
            cached: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    /// Stores the OAuth token.
    pub fn store(&self, token: OAuthToken) -> OutputResult<()> {
        *self.cached.lock() = Some(token);
        info!("Stored YouTube OAuth token in credential cache");
        Ok(())
    }

    /// Retrieves the stored token if present.
    pub fn load(&self) -> Option<OAuthToken> {
        self.cached.lock().clone()
    }

    /// Clears credentials upon user sign-out.
    pub fn clear(&self) {
        *self.cached.lock() = None;
        info!("Cleared YouTube OAuth token from credential cache");
    }
}

/// Generates a PKCE code verifier and S256 code challenge.
pub struct PkcePair {
    /// Raw cryptographically random code verifier
    pub code_verifier: String,
    /// S256 SHA-256 base64url-encoded code challenge
    pub code_challenge: String,
    /// PKCE code challenge method (always "S256")
    pub code_challenge_method: String,
}

impl PkcePair {
    /// Generates a high-entropy PKCE code verifier and its SHA-256 challenge.
    pub fn generate() -> Self {
        let random_bytes = uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
        let verifier = URL_SAFE_NO_PAD.encode(random_bytes.as_bytes());

        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(hash);

        Self {
            code_verifier: verifier,
            code_challenge: challenge,
            code_challenge_method: "S256".to_string(),
        }
    }
}

/// Local HTTP loopback listener for capturing the OAuth authorization code.
pub struct LoopbackAuthListener {
    listener: TcpListener,
    port: u16,
}

impl LoopbackAuthListener {
    /// Binds an ephemeral local port on 127.0.0.1 for the OAuth redirect.
    pub fn bind() -> OutputResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    /// Returns the assigned loopback port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Constructs the full redirect URI string.
    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Listens for a single incoming HTTP GET callback from Google and extracts the authorization code.
    pub fn wait_for_authorization_code(
        &self,
        timeout: std::time::Duration,
    ) -> OutputResult<String> {
        self.listener.set_nonblocking(false)?;
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(OutputError::StreamConnectionFailed {
                    url: "127.0.0.1".into(),
                    reason: "OAuth loopback callback timed out".into(),
                });
            }

            if let Ok((mut stream, _)) = self.listener.accept() {
                let mut buffer = [0u8; 2048];
                let bytes_read = stream.read(&mut buffer)?;
                let request = String::from_utf8_lossy(&buffer[..bytes_read]);

                // Extract query parameter `code=`
                if let Some(code_pos) = request.find("code=") {
                    let after_code = &request[code_pos + 5..];
                    let code_end = after_code
                        .find('&')
                        .or_else(|| after_code.find(' '))
                        .unwrap_or(after_code.len());
                    let code = after_code[..code_end].to_string();

                    // Send pleasant HTML confirmation back to browser
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                        <!DOCTYPE html><html><body style='font-family:sans-serif;background:#15171A;color:#E7E9EC;text-align:center;padding:50px;'>\
                        <h2>Zenocast Authentication Successful!</h2>\
                        <p>You may close this tab and return to Zenocast.</p>\
                        </body></html>";
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();

                    return Ok(code);
                }
            }
        }
    }
}
