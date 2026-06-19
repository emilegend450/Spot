use iced::{Application, Command as IcedCommand, Element, Theme, executor};
use iced::widget::{button, column, text, Container, TextInput};
use crate::api::spotify::{Spotify, TokenInfo};
use url::Url;
use is_wsl;
use std::fs::File;
use std::io::Write;
use std::env::temp_dir;
use std::process::Command as SysCommand;
use std::error::Error;

use crate::gui::sidebar::{self, Screen, SidebarMessage};
use crate::gui::screens;
use crate::gui::playback_bar;
use crate::theme::AppTheme;
use crate::settings;

#[derive(Debug, Clone)]
pub enum Message {
    /// Start the login process
    LoginRequested,
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
    /// Sidebar interaction (screen change or theme toggle)
    Sidebar(SidebarMessage),
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
    /// Currently active sidebar screen (only meaningful once logged in)
    active_screen: Screen,
    /// Currently active app theme (persisted to disk)
    app_theme: AppTheme,
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

    fn new(_flags: Self::Flags) -> (Self, IcedCommand<Message>) {
        let spotify = Spotify::new();
        (
            Self {
                spotify,
                status: StatusEnum::LoggedOut,
                auth_url: None,
                code_input: String::new(),
                token: None,
                error: None,
                active_screen: Screen::Search,
                app_theme: settings::load_theme(),
            },
            IcedCommand::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Spotix Lite")
    }

    fn update(&mut self, message: Self::Message) -> IcedCommand<Message> {
        match message {
            Message::LoginRequested => {
                // Generate the auth URL and show it
                let (auth_url, _csrf_state) = self.spotify.authorize_url();
                // Open the URL in the browser (WSL-aware)
                if is_wsl::is_wsl() {
                    // Debug: print the actual URL we're trying to open
                    println!("DEBUG: Auth URL: {}", auth_url);
                    // Use temp file approach to avoid & character issues in WSL
                    if let Err(e) = open_url_via_temp_file(&auth_url) {
                        println!("DEBUG: Temp file method failed: {}, trying cmd.exe with quoting", e);
                        // Fallback to properly quoted cmd.exe invocation
                        if let Err(e2) = open_url_via_cmd_quoted(&auth_url) {
                            self.error = Some(format!("Failed to open browser: {} / {}", e, e2));
                        } else {
                            println!("DEBUG: Successfully opened URL via quoted cmd.exe");
                        }
                    } else {
                        println!("DEBUG: Successfully opened URL via temp file method");
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
                IcedCommand::perform(
                    crate::api::callback_server::wait_for_callback(),
                    move |result| match result {
                        Ok((code, state)) => Message::TokenReceivedWithState(code, state),
                        Err(e) => Message::TokenFailed(e.to_string()),
                    }
                )
            }

            Message::CodeInputChanged(input) => {
                self.code_input = input;
                IcedCommand::none()
            }
            Message::TokenRequested => {
                // Extract code and state from the input (could be a full redirect URL or just the code)
                let (code, state) = extract_code_and_state(&self.code_input);
                if code.is_empty() {
                    self.error = Some("Please paste the full redirect URL or the authorization code.".to_string());
                    return IcedCommand::none();
                }
                // Exchange the code for a token with state verification
                let spotify_clone = self.spotify.clone();
                IcedCommand::perform(
                    handle_token_request_with_state(spotify_clone, code, state),
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
                IcedCommand::none()
            }
            Message::TokenReceivedWithState(code, state) => {
                // Exchange the code for a token using the verified state
                let spotify_clone = self.spotify.clone();
                IcedCommand::perform(
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
                IcedCommand::none()
            }
            Message::LogoutRequested => {
                self.token = None;
                self.status = StatusEnum::LoggedOut;
                self.auth_url = None;
                self.code_input.clear();
                IcedCommand::none()
            }
            Message::Sidebar(sidebar_msg) => {
                match sidebar_msg {
                    SidebarMessage::ScreenSelected(screen) => {
                        self.active_screen = screen;
                    }
                    SidebarMessage::ThemeToggleRequested => {
                        self.app_theme = self.app_theme.toggled();
                        settings::save_theme(self.app_theme);
                    }
                }
                IcedCommand::none()
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
            StatusEnum::LoggedIn => return self.view_shell(),
        };

        Container::new(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn theme(&self) -> Theme {
        self.app_theme.to_iced_theme()
    }
}

impl App {
    fn view_shell(&self) -> Element<'_, Message> {
        let sidebar_view = sidebar::view(self.active_screen, self.app_theme)
            .map(Message::Sidebar);

        let screen_content: Element<'_, Message> = match self.active_screen {
            Screen::Search => screens::search::view().map(|_| unreachable!()),
            Screen::Library => screens::library::view().map(|_| unreachable!()),
            Screen::MadeForYou => screens::made_for_you::view().map(|_| unreachable!()),
            Screen::Queue => screens::queue::view().map(|_| unreachable!()),
        };

        let bar = playback_bar::view().map(|_| unreachable!());

        let main_column = iced::widget::column![screen_content, bar]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill);

        iced::widget::row![sidebar_view, main_column]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }
}

/// Opens a URL using the temp file method (WSL-friendly)
fn open_url_via_temp_file(url: &str) -> std::io::Result<()> {
    // Create a temporary HTML file that redirects to the URL
    let temp_dir = temp_dir();
    let temp_file_path = temp_dir.join(format!(
        "spotix_redirect_{}.html",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    let html_content = format!(
        "<!DOCTYPE html>\\n<html>\\n<head>\\n<meta http-equiv=\\\"refresh\\\" content=\\\"0; url={}\\\">\\n<title>Redirecting to Spotify...</title>\\n</head>\\n<body>\\nIf you are not redirected automatically, follow this <a href=\\\"{}\\\">link</a>.\\n</body>\\n</html>",
        url, url
    );

    // Write the HTML file
    let mut file = File::create(&temp_file_path)?;
    file.write_all(html_content.as_bytes())?;

    // Convert the path to Windows format for explorer.exe
    let windows_path = if cfg!(target_os = "linux") && is_wsl::is_wsl() {
        // Use wslpath to convert Linux path to Windows path
        let output = SysCommand::new("wslpath")
            .arg("-w")
            .arg(&temp_file_path)
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other("wslpath failed"));
        }
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        temp_file_path.to_string_lossy().to_string()
    };

    // Open the temp file with explorer.exe
    SysCommand::new("explorer.exe")
        .arg(&windows_path)
        .spawn()?;

    Ok(())
}
fn open_url_via_cmd_quoted(url: &str) -> std::io::Result<()> {
    // Use PowerShell to start the URL, which handles quoting better in WSL
    // Escape single quotes in URL for PowerShell by doubling them
    let escaped_url = url.replace("'", "''");
    let ps_command = format!("Start-Process '{}'", escaped_url);
    SysCommand::new("powershell.exe")
        .args(["-NoProfile", "-Command", &ps_command])
        .spawn()?;
    Ok(())
}

/// Extracts the authorization code and state from a redirect URL or returns the input if it's already just the code.
fn extract_code_and_state(input: &str) -> (String, String) {
    // If the input contains 'code=', we try to extract the code and state parameters
    if let Ok(url) = Url::parse(input) {
        let code = url.query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .unwrap_or_default();
        let state = url.query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .unwrap_or_default();
        return (code, state);
    }
    // Otherwise, assume the input is the code itself (trim whitespace)
    (input.trim().to_string(), String::new())
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