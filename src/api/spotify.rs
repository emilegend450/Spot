use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use std::sync::Mutex;
use rand::{Rng, distributions::Alphanumeric};

/// Spotify client for handling OAuth and API requests.
#[derive(Debug, Clone)]
pub struct Spotify {
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    redirect_url: String,
    /// HTTP client for making requests to Spotify's token endpoint.
    http_client: Arc<reqwest::Client>,
    /// Token storage (in-memory for now, but could be persisted).
    pub token: Arc<tokio::sync::Mutex<Option<TokenInfo>>>,
    /// Pending OAuth state for CSRF protection.
    pending_state: Arc<Mutex<Option<String>>>,
}

impl Default for Spotify {
    fn default() -> Self {
        Self::new()
    }
}

impl Spotify {
    /// Create a new Spotify client from environment variables.
    pub fn new() -> Self {
        let client_id = env::var("SPOTIFY_CLIENT_ID").expect("SPOTIFY_CLIENT_ID must be set");
        let client_secret = env::var("SPOTIFY_CLIENT_SECRET").expect("SPOTIFY_CLIENT_SECRET must be set");

        // Spotify's OAuth endpoints
        let auth_url = "https://accounts.spotify.com/authorize".to_string();
        let token_url = "https://accounts.spotify.com/api/token".to_string();

        // Set up the redirect URL (we'll use a local port)
        let redirect_url = "http://127.0.0.1:8080/callback".to_string();

        let http_client = Arc::new(reqwest::Client::new());

        Self {
            client_id,
            client_secret,
            auth_url,
            token_url,
            redirect_url,
            http_client,
            token: Arc::new(tokio::sync::Mutex::new(None)),
            pending_state: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new Spotify client with custom parameters (for testing).
    #[doc(hidden)]
    pub fn new_with_params(client_id: String, client_secret: String, auth_url: String, token_url: String, redirect_url: String) -> Self {
        let http_client = Arc::new(reqwest::Client::new());

        Self {
            client_id,
            client_secret,
            auth_url,
            token_url,
            redirect_url,
            http_client,
            token: Arc::new(tokio::sync::Mutex::new(None)),
            pending_state: Arc::new(Mutex::new(None)),
        }
    }

    /// Generate the authorization URL to start the OAuth flow.
    pub fn authorize_url(&self) -> (String, String) {
        let state: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let scope = [
            "user-read-private",
            "user-read-email",
            "streaming",
            "user-modify-playback-state",
            "user-read-playback-state",
            "playlist-read-private",
            "playlist-modify-public",
            "playlist-modify-private",
            "user-library-read",
            "user-library-modify",
        ]
        .join(" ");

        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            self.auth_url,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_url),
            urlencoding::encode(&scope),
            urlencoding::encode(&state)
        );

        // Store the generated state for CSRF protection
        *self.pending_state.lock().unwrap() = Some(state.clone());

        (auth_url, state)
    }

    /// Handle the callback from Spotify and exchange the code for a token.
    pub async fn handle_callback(&self, code: String, state: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Verify the state to prevent CSRF attacks
        let expected_state = self.pending_state.lock().unwrap().take()
            .ok_or("No pending state — authorize first")?;
        if state != expected_state {
            return Err("State mismatch — possible CSRF attack".into());
        }

        // Exchange the code for an access token
        let token_info = self.http_client
            .post(&self.token_url)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", &self.redirect_url),
            ])
            .send()
            .await?
            .json::<TokenInfo>()
            .await?;

        // Store the token
        let mut token_lock = self.token.lock().await;
        *token_lock = Some(token_info);
        Ok(())
    }

    /// Get the current token (if any).
    pub async fn token(&self) -> Option<TokenInfo> {
        let token_lock = self.token.lock().await;
        token_lock.clone()
    }

    /// Refresh the access token using the refresh token.
    pub async fn refresh_access_token(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let refresh_token = {
            let lock = self.token.lock().await;
            lock.as_ref()
                .and_then(|t| t.refresh_token.clone())
                .ok_or("No refresh token stored")?
        };

        let new_token = self.http_client
            .post(&self.token_url)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh_token),
            ])
            .send()
            .await?
            .json::<TokenInfo>()
            .await?;

        let mut lock = self.token.lock().await;
        *lock = Some(new_token);
        Ok(())
    }
    /// Generic GET request to the Spotify API
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        let token = self.token.lock().await;
        let token = token.as_ref()
            .ok_or_else(|| "No token available".to_string())?;
        let url = format!("https://api.spotify.com/v1/{}", endpoint);
        let response = self.http_client
            .get(url)
            .bearer_auth(token.access_token.as_str())
            .send()
            .await?
            .error_for_status()?;
        let json = response.json().await?;
        Ok(json)
    }

    /// Get the current user's profile
    pub async fn current_user(&self) -> Result<CurrentUser, Box<dyn std::error::Error + Send + Sync>> {
        self.get("me").await
    }
    /// Get the currently playing track
    pub async fn currently_playing(&self) -> Result<Option<CurrentlyPlaying>, Box<dyn std::error::Error + Send + Sync>> {
        let token = self.token.lock().await;
        let token = token.as_ref()
            .ok_or_else(|| "No token available".to_string())?;
        let url = "https://api.spotify.com/v1/me/player/currently-playing";
        let response = self.http_client
            .get(url)
            .bearer_auth(token.access_token.as_str())
            .send()
            .await?;
        if response.status().as_u16() == 204 {
            return Ok(None);
        }
        let json = response.json().await?;
        Ok(Some(json))
    }

    /// Get the current user's playlists
    pub async fn user_playlists(&self, limit: usize) -> Result<Page<SimplePlaylist>, Box<dyn std::error::Error + Send + Sync>> {
        self.get(format!("me/playlists?limit={}", limit).as_str()).await
    }
    /// Pause playback
    pub async fn pause_playback(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.token.lock().await;
        let token = token.as_ref()
            .ok_or_else(|| "No token available".to_string())?;
        let url = "https://api.spotify.com/v1/me/player/pause";
        self.http_client
            .put(url)
            .bearer_auth(token.access_token.as_str())
            .send()
            .await?;
        Ok(())
    }

    /// Resume playback
    pub async fn resume_playback(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.token.lock().await;
        let token = token.as_ref()
            .ok_or_else(|| "No token available".to_string())?;
        let url = "https://api.spotify.com/v1/me/player/play";
        self.http_client
            .put(url)
            .bearer_auth(token.access_token.as_str())
            .send()
            .await?;
        Ok(())
    }

    /// Skip to next track
    pub async fn next_track(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.token.lock().await;
        let token = token.as_ref()
            .ok_or_else(|| "No token available".to_string())?;
        let url = "https://api.spotify.com/v1/me/player/next";
        self.http_client
            .post(url)
            .bearer_auth(token.access_token.as_str())
            .send()
            .await?;
        Ok(())
    }

    /// Skip to previous track
    pub async fn previous_track(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.token.lock().await;
        let token = token.as_ref()
            .ok_or_else(|| "No token available".to_string())?;
        let url = "https://api.spotify.com/v1/me/player/previous";
        self.http_client
            .post(url)
            .bearer_auth(token.access_token.as_str())
            .send()
            .await?;
        Ok(())
    }
}

/// Struct to hold the token response from Spotify (for serialization if needed).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenInfo {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u32,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

// User profile

#[derive(Deserialize, Debug, Clone)]
pub struct CurrentUser {
    pub display_name: String,
    pub email: String,
    pub id: String,
    pub product: String,
}

// Simplified playlist for listing

#[derive(Deserialize, Debug, Clone)]
pub struct SimplePlaylist {
    pub name: String,
    pub id: String,
    pub images: Vec<PlaylistImage>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlaylistImage {
    pub url: String,
}

// Paginated response from Spotify API

#[derive(Deserialize, Debug, Clone)]
pub struct Page<T> {
    pub href: String,
    pub items: Vec<T>,
    pub limit: i32,
    pub next: Option<String>,
    pub offset: i32,
    pub previous: Option<String>,
    pub total: i32,
}

/// Currently playing track
#[derive(Deserialize, Debug, Clone)]
pub struct CurrentlyPlaying {
    pub item: Option<Track>,
    pub is_playing: bool,
    pub progress_ms: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration_ms: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Artist {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Album {
    pub name: String,
    pub images: Vec<Image>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Image {
    pub url: String,
}
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Helper function to create a canned token response
    fn create_token_response() -> String {
        r#"{
            "access_token": "test_access_token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "test_refresh_token",
            "scope": "user-read-private user-read-email"
        }"#.to_string()
    }

    /// Mock token server that serves the canned token response
    /// Returns the assigned port, join handle and a receiver that signals when the server is ready
    async fn start_mock_token_server() -> (u16, tokio::task::JoinHandle<()>, tokio::sync::mpsc::Receiver<()>) {
        // Bind to port 0 to get a random available port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let addr = format!("127.0.0.1:{port}");
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn(async move {
            println!("Mock token server listening on {addr}");
            let _ = tx.send(()).await;

            // Small delay to ensure the test has time to make the connection after we signal ready
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Accept one connection and serve the token response
            match listener.accept().await {
                Ok((mut socket, _)) => {
                    // Read the request (we don't need it, but we must read it to avoid leaving the client hanging)
                    let mut buf = [0u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let response = create_token_response();
                    let http_response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        response.len(),
                        response
                    );

                    let _ = socket.write_all(http_response.as_bytes()).await;
                    let _ = socket.flush().await;
                }
                Err(e) => {
                    eprintln!("Accept failed: {e}");
                }
            }
        });
        (port, handle, rx)
    }

    #[tokio::test]
    async fn test_oauth_flow_success() {
        // Set up test Spotify client with mock endpoints
        let spotify = Spotify::new_with_params(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            "https://accounts.spotify.com/authorize".to_string(),
            "http://127.0.0.1:8081/token".to_string(), // Custom token URL for testing (will be overridden)
            "http://127.0.0.1:8080/callback".to_string(),
        );

        // Generate auth URL and extract state
        let (auth_url, state_from_url) = spotify.authorize_url();
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains(&state_from_url));

        // Start mock token server and wait for it to be ready
        let (mock_port, _server_handle, mut ready_rx) = start_mock_token_server().await;
        // Update the spotify client's token URL to use the actual port
        let spotify_with_port = Spotify::new_with_params(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            "https://accounts.spotify.com/authorize".to_string(),
            format!("http://127.0.0.1:{mock_port}/token"),
            "http://127.0.0.1:8080/callback".to_string(),
        );
        // Set the pending state to the one we generated from the original authorize_url call
        *spotify_with_port.pending_state.lock().unwrap() = Some(state_from_url.clone());
        // Wait for the server to signal it's ready
        let _ = ready_rx.recv().await;

        // Simulate callback with matching state
        let test_code = "test_auth_code".to_string();
        let result = spotify_with_port.handle_callback(test_code.clone(), state_from_url).await;

        // Should succeed
        assert!(result.is_ok(), "Handle callback should succeed with matching state: {:?}", result);

        // Verify token was stored
        let token = spotify_with_port.token().await;
        assert!(token.is_some(), "Token should be stored after successful callback");
        let token_info = token.unwrap();
        assert_eq!(token_info.access_token, "test_access_token");
        assert_eq!(token_info.token_type, "Bearer");
        assert_eq!(token_info.expires_in, 3600);
        assert_eq!(token_info.refresh_token.as_deref(), Some("test_refresh_token"));
        assert_eq!(token_info.scope.as_deref(), Some("user-read-private user-read-email"));
    }

    #[tokio::test]
    async fn test_oauth_flow_csrf_failure() {
        // Set up test Spotify client
        let spotify = Spotify::new_with_params(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            "https://accounts.spotify.com/authorize".to_string(),
            "http://127.0.0.1:8082/token".to_string(), // Custom token URL for testing (will be overridden)
            "http://127.0.0.1:8080/callback".to_string(),
        );

        // Generate auth URL to set up pending state
        let (_auth_url, state_from_url) = spotify.authorize_url();

        // Start mock token server (won't be reached due to CSRF failure)
        let (mock_port, _server_handle, mut ready_rx) = start_mock_token_server().await;
        // Update the spotify client's token URL to use the actual port
        let spotify_with_port = Spotify::new_with_params(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            "https://accounts.spotify.com/authorize".to_string(),
            format!("http://127.0.0.1:{mock_port}/token"),
            "http://127.0.0.1:8080/callback".to_string(),
        );
        // Set the pending state to the one we generated from the original authorize_url call
        *spotify_with_port.pending_state.lock().unwrap() = Some(state_from_url);
        // Wait for the server to signal it's ready (though we won't reach it)
        let _ = ready_rx.recv().await;

        // Simulate callback with mismatched state
        let test_code = "test_auth_code".to_string();
        let wrong_state = "wrong_state_value".to_string();
        let result = spotify_with_port.handle_callback(test_code, wrong_state).await;

        // Should fail with CSRF error
        assert!(result.is_err(), "Handle callback should fail with mismatched state");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("State mismatch"), "Error should indicate CSRF attack: {}", err_msg);

        // Verify no token was stored
        let token = spotify_with_port.token().await;
        assert!(token.is_none(), "No token should be stored after CSRF failure");
    }

    #[tokio::test]
    async fn test_oauth_flow_missing_state() {
        // Set up test Spotify client
        let spotify = Spotify::new_with_params(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            "https://accounts.spotify.com/authorize".to_string(),
            "http://127.0.0.1:8083/token".to_string(), // Custom token URL for testing (will be overridden)
            "http://127.0.0.1:8080/callback".to_string(),
        );

        // Manually clear the pending state to simulate missing state
        *spotify.pending_state.lock().unwrap() = None;

        // Start mock token server (won't be reached)
        let (mock_port, _server_handle, mut ready_rx) = start_mock_token_server().await;
        // Update the spotify client's token URL to use the actual port
        let spotify_with_port = Spotify::new_with_params(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            "https://accounts.spotify.com/authorize".to_string(),
            format!("http://127.0.0.1:{mock_port}/token"),
            "http://127.0.0.1:8080/callback".to_string(),
        );
        // Wait for the server to signal it's ready (though we won't reach it)
        let _ = ready_rx.recv().await;

        // Simulate callback with any state (should fail due to missing pending state)
        let test_code = "test_auth_code".to_string();
        let test_state = "any_state".to_string();
        let result = spotify_with_port.handle_callback(test_code, test_state).await;

        // Should fail with "No pending state" error
        assert!(result.is_err(), "Handle callback should fail with missing pending state");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No pending state"), "Error should indicate missing state: {}", err_msg);
    }
}