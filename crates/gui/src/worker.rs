use crate::message::Message;
use iced::futures::{self, SinkExt, channel::mpsc::Sender};
use openhx_core::{HxError, is_device_present, with_device};
use std::time::Duration;
use tracing::warn;

/// Creates an asynchronous stream that actively polls for a connected device.
///
/// `HxError::DeviceNotFound` is treated as the "still waiting" steady state and
/// silently retried; any other error means we got far enough to interact with
/// the device but something failed (claim refused, handshake error, malformed
/// preset stream, …) and is surfaced to the UI so the user isn't stuck on a
/// blank "Waiting" screen.
pub fn usb_poll() -> impl futures::Stream<Item = Message> {
    iced::stream::channel(1, |mut output: Sender<Message>| async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));

        loop {
            interval.tick().await;

            let result = tokio::task::spawn_blocking(|| {
                with_device(|client| {
                    let name = client.profile().name.to_string();
                    let presets = client.read_presets()?;
                    Ok((name, presets))
                })
            })
            .await;

            match result {
                Ok(Ok((name, presets))) => {
                    let _ = output.send(Message::DeviceDetected(name, presets)).await;
                    break;
                }
                Ok(Err(HxError::DeviceNotFound)) => continue,
                Ok(Err(e)) => {
                    let _ = output.send(Message::ConnectionError(e.to_string())).await;
                    break;
                }
                Err(e) => {
                    warn!("usb poll task panicked: {e}");
                    continue;
                }
            }
        }
    })
}

/// Creates an asynchronous stream that monitors an established connection for
/// dropouts using a non-claiming USB enumeration probe.
pub fn usb_check_disconnect() -> impl futures::Stream<Item = Message> {
    iced::stream::channel(1, |mut output: Sender<Message>| async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));

        loop {
            interval.tick().await;

            let present = match tokio::task::spawn_blocking(is_device_present).await {
                Ok(p) => p,
                Err(e) => {
                    warn!("usb presence-check task panicked: {e}");
                    false
                }
            };

            if !present {
                let _ = output.send(Message::DeviceDisconnected).await;
                break;
            }
        }
    })
}
