use std::io::{Error, ErrorKind};

use anyhow::Result;
use esp_idf_svc::{mqtt::client::EventPayload, sys::EspError, timer::EspAsyncTimer};

use crate::{context::Context, handle_event_implementation};

pub async fn handle_event<'a>(
    esp_async_timer: &mut EspAsyncTimer,
    event_payload: EventPayload<'a, EspError>,
    context: Context,
) -> Result<()> {
    match event_payload {
        EventPayload::Received {
            id: _,
            topic,
            data,
            details: _,
        } => match topic {
            Some(definitely_topic) => {
                handle_event_implementation(definitely_topic, esp_async_timer, data, context)
                    .await?
            }
            None => Err(Error::new(
                ErrorKind::InvalidData,
                "Received message: Topic not defined.",
            ))?,
        },
        _ => {
            let message = format!(
                "Received message: Payload status is not `EventPayload::Received`, instead: {}",
                event_payload
            );
            Err(Error::new(ErrorKind::InvalidData, message))?
        }
    };
    Ok(())
}
