use iced::{Application, Settings};
use is_wsl::is_wsl;
use spotix_lite::gui::App;
use std::env;
use std::fs;

fn main() -> iced::Result {
    println!("Starting...");
    // Set environment variables for WSL to improve GUI compatibility
    if is_wsl() {
        unsafe {
            env::set_var("WINIT_UNIX_BACKEND", "x11");
            env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
        }
        println!("Set WSL environment variables: WINIT_UNIX_BACKEND=x11, LIBGL_ALWAYS_SOFTWARE=1");
    }
    println!("Current directory: {:?}", env::current_dir().unwrap());
    // Check if .env file exists
    if let Ok(metadata) = fs::metadata(".env") {
        println!(".env file exists, size: {}", metadata.len());
    } else {
        println!(".env file does not exist in current directory");
    }
    // Read the .env file, replace escaped newlines, and parse
    if let Ok(contents) = fs::read_to_string(".env") {
        println!("Raw .env contents (first 200 chars): {:?}", &contents[..std::cmp::min(contents.len(), 200)]);
        // Replace escaped newlines (\\n) with actual newlines
        let contents = contents.replace("\\n", "\n");
        println!("After replacing escaped newlines (first 200 chars): {:?}", &contents[..std::cmp::min(contents.len(), 200)]);
        // Parse line by line
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if !key.is_empty() && !value.is_empty() {
                    unsafe { env::set_var(key, value); }
                    // Redact sensitive information
                    let display_value = if key.to_uppercase().contains("SECRET") || key.to_uppercase().contains("TOKEN") {
                        "[REDACTED]"
                    } else {
                        value
                    };
                    println!("Set {}={}", key, display_value);
                }
            }
        }
    } else {
        println!("Failed to read .env file");
    }
    // Check the environment variables after loading
    let client_id = env::var("SPOTIFY_CLIENT_ID");
    let client_secret = env::var("SPOTIFY_CLIENT_SECRET");
    println!("AFTER LOAD - SPOTIFY_CLIENT_ID: {:?}", client_id);
    println!("AFTER LOAD - SPOTIFY_CLIENT_SECRET: {:?}", client_secret.map(|_| "set"));
    println!("Before running App");
    App::run(Settings::default())
}