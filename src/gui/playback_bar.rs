use iced::widget::{row, text, Container};
use iced::{Element, Length};

#[derive(Debug, Clone)]
pub enum PlaybackBarMessage {}

pub fn view() -> Element<'static, PlaybackBarMessage> {
    Container::new(
        row![
            text("No track playing").size(13),
        ]
        .spacing(12)
        .padding(12)
    )
    .width(Length::Fill)
    .height(Length::Fixed(64.0))
    .into()
}