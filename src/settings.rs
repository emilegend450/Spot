use crate::theme::AppTheme;
use std::path::PathBuf;
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SettingsFile {
    theme: AppTheme,
}

fn settings_path() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
    dir.push("spotix-lite");
    let _ = fs::create_dir_all(&dir);
    dir.push("settings.json");
    dir
}

pub fn load_theme() -> AppTheme {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str::<SettingsFile>(&contents)
            .map(|s| s.theme)
            .unwrap_or_default(),
        Err(_) => AppTheme::default(),
    }
}

pub fn save_theme(theme: AppTheme) {
    let path = settings_path();
    let data = SettingsFile { theme };
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = fs::write(path, json);
    }
}