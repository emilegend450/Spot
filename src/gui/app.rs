use iced::{Application, Command, Element, Theme, executor};
use iced::widget::{button, column, text, Container, TextInput};
use crate::api::spotify::{Spotify, TokenInfo};
use url::Url;
use std::error::Error;
use is_wsl;

#[derive(Debug, Clone)]
pub enum Message {
    /// Start the login process
    LoginRequested,
    /// Open the authorization URL in the browser
    AuthUrlGenerated(String),
    /// User has pasted a redirect URL or code
    CodeInputChanged(String),
    /// Exchange the code for a token (manual entry)
    TokenRequested,
    /// Received code and state from automatic callback listener
    TokenReceivedWithState(String, String),
    /// Token received successfully
    TokenReceived(TokenInfo),
    /// Failed to get token
    TokenFailed(String),
    /// Log out
    LogoutRequested,
}

pub struct App {
    /// Spotify client for handling OAuth and API requests
    spotify: Spotify,
    /// Current application state
    status: StatusEnum,
    /// The authorization URL (if we have generated one)
    auth_url: Option<String>,
    /// The code input from the user (if pasting a redirect URL or code)
    code_input: String,
    /// The token if we have one
    token: Option<TokenInfo>,
    /// Error message if any
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum StatusEnum {
    LoggedOut,
    LoggingIn { auth_url: String },
    LoggedIn,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        let spotify = Spotify::new();
        (
            Self {
                spotify,
                status: StatusEnum::LoggedOut,
                auth_url: None,
                code_input: String::new(),
                token: None,
                error: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Spotix Lite")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::LoginRequested => {
                // Generate the auth URL and show it
                let (auth_url, _csrf_state) = self.spotify.authorize_url();
                // Open the URL in the browser (WSL-aware)
                if is_wsl::is_wsl() {
                    if let Err(e) = std::process::Command::new("cmd.exe")
                        .args(["/c", "start", "", &auth_url])
                        .spawn()
                    {
                        self.error = Some(format!("Failed to open browser: {e}"));
                    }
                } else {
                    if let Err(e) = open::that(&auth_url) {
                        self.error = Some(format!("Failed to open browser: {e}"));
                    }
                }
                self.status = StatusEnum::LoggingIn { auth_url: auth_url.clone() };
                self.auth_url = Some(auth_url);
                // Start waiting for the callback automatically
                let _spotify_clone = self.spotify.clone();
                Command::perform(
                    crate::api::callback_server::wait_for_callback(),
                    move |result| match result {
                        Ok((code, state)) => Message::TokenReceivedWithState(code, state),
                        Err(e) => Message::TokenFailed(e.to_string()),
                    }
                )
            }
            Message::AuthUrlGenerated(url) => {
                self.auth_url = Some(url.clone());
                self.status = StatusEnum::LoggingIn { auth_url: url };
                Command::none()
            }
            Message::CodeInputChanged(input) => {
                self.code_input = input;
                Command::none()
            }
            Message::TokenRequested => {
                // Extract code from the input (could be a full redirect URL or just the code)
                let code = extract_code(&self.code_input);
                if code.is_empty() {
                    self.error = Some("Please paste the full redirect URL or the authorization code.".to_string());
                    return Command::none();
                }
                // Exchange the code for a token
                let _spotify_clone = self.spotify.clone();
                // We'll perform the token request asynchronously
                Command::perform(
                    handle_token_request(_spotify_clone, code),
                    |result| match result {
                        Ok(token_info) => Message::TokenReceived(token_info),
                        Err(e) => Message::TokenFailed(e.to_string()),
                    }
                )
            }
            Message::TokenReceived(token_info) => {
                self.token = Some(token_info.clone());
                self.status = StatusEnum::LoggedIn;
                self.error = None;
                Command::none()
            }
            Message::TokenReceivedWithState(code, state) => {
                // Exchange the code for a token using the verified state
                let spotify_clone = self.spotify.clone();
                Command::perform(
                    handle_token_request_with_state(spotify_clone, code, state),
                    |result| match result {
                        Ok(token_info) => Message::TokenReceived(token_info),
                        Err(e) => Message::TokenFailed(e.to_string()),
                    }
                )
            }
            Message::TokenFailed(err) => {
                self.error = Some(format!("Failed to get token: {err}"));
                self.status = StatusEnum::LoggedOut;
                Command::none()
            }
            Message::LogoutRequested => {
                self.token = None;
                self.status = StatusEnum::LoggedOut;
                self.auth_url = None;
                self.code_input.clear();
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match &self.status {
            StatusEnum::LoggedOut => column![
                text("Welcome to Spotix Lite!").size(30),
                button("Login with Spotify")
                    .on_press(Message::LoginRequested)
                    .padding(10),
                self.error.as_ref().map_or_else(|| text(""), |err| text(err).style(iced::theme::Text::Color([1.0, 0.0, 0.0].into())))
            ]
            .padding(20)
            .align_items(iced::Alignment::Center),
            StatusEnum::LoggingIn { auth_url } => column![
                text("Please authorize Spotix Lite in your browser:").size(20),
                text(auth_url).size(16),
                text("After authorizing, you will be redirected to a URL like:").size(16),
                text("http://127.0.0.1:8080/callback?code=....&state=....").size(16),
                text("Paste the full redirect URL or just the code here:").size(16),
                TextInput::new("Paste here...", &self.code_input)
                    .on_input(Message::CodeInputChanged)
                    .padding(10),
                button("Submit Code")
                    .on_press(Message::TokenRequested)
                    .padding(10),
                self.error.as_ref().map_or_else(|| text(""), |err| text(err).style(iced::theme::Text::Color([1.0, 0.0, 0.0].into())))
            ]
            .padding(20)
            .align_items(iced::Alignment::Start),
            StatusEnum::LoggedIn => column![
                text("Logged in successfully!").size(20),
                text("Logged in successfully! Token acquired.").size(16),
                button("Logout")
                    .on_press(Message::LogoutRequested)
                    .padding(10),
            ]
            .padding(20)
            .align_items(iced::Alignment::Center),
        };

        Container::new(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::default()
    }
}

/// Extracts the authorization code from a redirect URL or returns the input if it's already just the code.
fn extract_code(input: &str) -> String {
    // If the input contains 'code=', we try to extract the code parameter
    if let Ok(url) = Url::parse(input) {
        return url.query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .unwrap_or_default();
    }
    // Otherwise, assume the input is the code itself (trim whitespace)
    input.trim().to_string()
}

/// Asynchronously handles the token request: exchanges the code for a token and retrieves it.
async fn handle_token_request(spotify: Spotify, code: String) -> Result<TokenInfo, Box<dyn Error + Send + Sync>> {
    // Exchange the code for a token
    spotify.handle_callback(code, String::new()).await?;
    // Get the token from the client
    let token = spotify
        .token()
        .await
        .ok_or_else(|| Box::<dyn Error + Send + Sync>::from("No token found after callback"))?;
    Ok(token)
}

/// Asynchronously handles the token request with state verification: exchanges the code for a token and retrieves it.
async fn handle_token_request_with_state(spotify: Spotify, code: String, state: String) -> Result<TokenInfo, Box<dyn Error + Send + Sync>> {
    // Exchange the code for a token with state verification
    spotify.handle_callback(code, state).await?;
    // Get the token from the client
    let token = spotify
        .token()
        .await
        .ok_or_else(|| Box::<dyn Error + Send + Sync>::from("No token found after callback"))?;
    Ok(token)
}