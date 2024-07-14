use std::io::{Error, ErrorKind};

use anyhow::Result;
use esp_idf_svc::{mqtt::client::EventPayload, sys::EspError, timer::EspAsyncTimer};
use log::info;

use crate::{context::Context, handle_event_implementation::handle_event_implementation};

pub async fn handle_event<'a>(
    timer: &mut EspAsyncTimer,
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
                handle_event_implementation(timer, definitely_topic, data, context).await?
            }
            None => Err(Error::new(
                ErrorKind::InvalidData,
                "Received message: Topic not defined.",
            ))?,
        },
        _ => {
            info!(
                "Received message: Payload status is not `EventPayload::Received`, instead: {}",
                event_payload
            );
        }
    };
    Ok(())
}
