use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppTheme {
    #[default]
    PinkLofi,
    DarkLofi,
}

impl AppTheme {
    pub fn to_iced_theme(self) -> iced::Theme {
        match self {
            AppTheme::PinkLofi => iced::Theme::Light,
            AppTheme::DarkLofi => iced::Theme::Dark,
        }
    }
    pub fn toggled(self) -> Self {
        match self {
            AppTheme::PinkLofi => AppTheme::DarkLofi,
            AppTheme::DarkLofi => AppTheme::PinkLofi,
        }
    }
}

