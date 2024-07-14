use std::io::{Error, ErrorKind};

use anyhow::Result;

use crate::{
    context::Context,
    handler_functions::{
        handle_change_charging_speed, handle_start_charging, handle_start_trip,
        handle_stop_charging,
    },
};

pub fn handle_event_implementation<'a>(topic: &str, data: &[u8], context: Context) -> Result<()> {
    match topic {
        "/charging-controller/start-charging" => handle_start_charging(data, context),
        "/charging-controller/change-charging-speed" => handle_change_charging_speed(data, context),
        "/charging-controller/stop-charging" => handle_stop_charging(data, context),
        "/charging-controller/start-trip" => handle_start_trip(data, context),
        _ => {
            let message = format!("Topic: {topic} not available");
            Err(Error::new(ErrorKind::InvalidData, message))?
        }
    }
}
