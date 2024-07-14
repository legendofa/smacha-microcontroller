use std::io::{Error, ErrorKind};

use anyhow::Result;
use esp_idf_svc::timer::EspAsyncTimer;

use crate::{
    context::Context,
    handler_functions::{
        handle_change_charging_speed, handle_start_charging, handle_start_trip,
        handle_stop_charging,
    },
};

pub async fn handle_event_implementation<'a>(
    timer: &mut EspAsyncTimer,
    topic: &str,
    data: &[u8],
    context: Context,
) -> Result<()> {
    match topic {
        "/charging-controller/start-charging" => handle_start_charging(timer, data, context).await,
        "/charging-controller/change-charging-speed" => {
            handle_change_charging_speed(timer, data, context).await
        }
        "/charging-controller/stop-charging" => handle_stop_charging(timer, data, context).await,
        "/charging-controller/start-trip" => handle_start_trip(timer, data, context).await,
        _ => {
            let message = format!("Topic: {topic} not available");
            Err(Error::new(ErrorKind::InvalidData, message))?
        }
    }
}
