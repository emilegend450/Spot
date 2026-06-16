use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
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
    pending_state: Arc<tokio::sync::Mutex<Option<String>>>,
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
            pending_state: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Generate the authorization URL to start the OAuth flow.
    pub fn authorize_url(&self) -> (String, String) {
        let state: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let scope = vec![
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

        (auth_url, state)
    }

    /// Handle the callback from Spotify and exchange the code for a token.
    pub async fn handle_callback(&self, code: String, state: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Verify the state to prevent CSRF attacks
        let expected_state = self.pending_state.lock().await.take()
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