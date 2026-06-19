use iced::widget::{column, text, Container};
use iced::{Element, Length};

#[derive(Debug, Clone)]
pub enum LibraryMessage {}

pub fn view() -> Element<'static, LibraryMessage> {
    Container::new(
        column![text("Library").size(24), text("Coming soon.").size(14)]
            .spacing(8)
            .padding(20)
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}