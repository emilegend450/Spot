use iced::widget::{row, text, Container, Button, Space};
use iced::{Element, Length, Alignment};

use crate::api::spotify::CurrentlyPlaying;

#[derive(Debug, Clone)]
pub enum PlaybackBarMessage {
    PlayPause,
    NextTrack,
    PreviousTrack,
}

pub fn view(playback_state: Option<&CurrentlyPlaying>) -> Element<'_, PlaybackBarMessage> {
    let content = match playback_state {
        Some(track) => {
            let track_info = format!(
                "{} - {}",
                track.item.as_ref().map(|i| i.name.clone()).unwrap_or_default(),
                track.item.as_ref().and_then(|i| i.artists.first()).map(|a| a.name.clone()).unwrap_or_default()
            );
            row![
                text(track_info).size(16),
                Space::with_width(Length::Fill),
                Button::new(text(if track.is_playing { "⏸" } else { "▶" }))
                    .on_press(PlaybackBarMessage::PlayPause)
                    .padding(5),
                Button::new(text("⏭"))
                    .on_press(PlaybackBarMessage::NextTrack)
                    .padding(5),
                Button::new(text("⏮"))
                    .on_press(PlaybackBarMessage::PreviousTrack)
                    .padding(5),
            ]
            .spacing(10)
            .align_items(Alignment::Center)
        }
        None => {
            row![
                text("No track playing").size(13),
            ]
            .spacing(12)
            .padding(12)
        }
    };

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fixed(64.0))
        .into()
}