use anyhow::Result;
use esp_idf_svc::timer::EspAsyncTimer;
use serde::Deserialize;

use crate::context::Context;

#[derive(Deserialize, Debug)]
pub struct ChargingEventData {
    charging_speed_w: u32,
}

#[derive(Deserialize, Debug)]
struct StartTripEventData {
    energy_usage_w: u32,
}

pub async fn handle_start_charging(data: &[u8], context: Context) -> Result<()> {
    let charging_event_data: ChargingEventData = serde_json::from_slice(data)?;
    let mut charging_controller = context.charging_controller_mutex.lock().unwrap();
    charging_controller.start_charging(charging_event_data.charging_speed_w)?;
    Ok(())
}

pub async fn handle_change_charging_speed(data: &[u8], context: Context) -> Result<()> {
    let charging_event_data: ChargingEventData = serde_json::from_slice(data)?;
    let mut charging_controller = context.charging_controller_mutex.lock().unwrap();
    charging_controller.change_charging_speed(charging_event_data.charging_speed_w)?;
    Ok(())
}

pub async fn handle_stop_charging(_data: &[u8], context: Context) -> Result<()> {
    let mut charging_controller = context.charging_controller_mutex.lock().unwrap();
    charging_controller.stop_charging()?;
    Ok(())
}

pub async fn handle_start_trip(data: &[u8], context: Context) -> Result<()> {
    let start_trip_event_data: StartTripEventData = serde_json::from_slice(data)?;
    let mut car = context.car_rwlock.write().unwrap();
    Ok(())
}
