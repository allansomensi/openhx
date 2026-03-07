use crate::message::Message;
use iced::futures::{self, SinkExt, channel::mpsc::Sender};
use openhx_core::Client;
use std::time::Duration;

/// Creates an asynchronous stream that actively polls for a connected device.
pub fn usb_poll() -> impl futures::Stream<Item = Message> {
    iced::stream::channel(1, |mut output: Sender<Message>| async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));

        loop {
            interval.tick().await;

            let result = tokio::task::spawn_blocking(|| match Client::detect() {
                Ok(client) => {
                    let name = client.profile().name.to_string();
                    match client.read_presets() {
                        Ok(presets) => Some(Message::DeviceDetected(name, presets)),
                        Err(e) => Some(Message::ConnectionError(e.to_string())),
                    }
                }
                Err(_) => None,
            })
            .await;

            if let Ok(Some(msg)) = result {
                let _ = output.send(msg).await;

                break;
            }
        }
    })
}

/// Creates an asynchronous stream that monitors an established connection for dropouts.
pub fn usb_check_disconnect() -> impl futures::Stream<Item = Message> {
    iced::stream::channel(1, |mut output: Sender<Message>| async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));

        loop {
            interval.tick().await;

            let is_disconnected = tokio::task::spawn_blocking(|| Client::detect().is_err())
                .await
                .unwrap_or(true);

            if is_disconnected {
                let _ = output.send(Message::DeviceDisconnected).await;
                break;
            }
        }
    })
}
