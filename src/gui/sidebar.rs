use iced::widget::{button, column, text, Container};
use iced::{Element, Length};
use crate::theme::AppTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Search,
    Library,
    MadeForYou,
    Queue,
}

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    ScreenSelected(Screen),
    ThemeToggleRequested,
}

pub fn view(active: Screen, _current_theme: AppTheme) -> Element<'static, SidebarMessage> {
    let nav_button = |label: &'static str, screen: Screen, active: Screen| {
        button(text(label).size(14))
            .on_press(SidebarMessage::ScreenSelected(screen))
            .width(Length::Fill)
            .padding(10)
            .style(if screen == active {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Text
            })
    };

    let content = column![
        text("Spotix Lite").size(18),
        nav_button("Search", Screen::Search, active),
        nav_button("Library", Screen::Library, active),
        nav_button("Made For You", Screen::MadeForYou, active),
        nav_button("Queue", Screen::Queue, active),
        button(text("Toggle Theme").size(14))
            .on_press(SidebarMessage::ThemeToggleRequested)
            .width(Length::Fill)
            .padding(10),
    ]
    .spacing(8)
    .padding(16)
    .width(Length::Fixed(200.0));

    Container::new(content)
        .height(Length::Fill)
        .into()
}