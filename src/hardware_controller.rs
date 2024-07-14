use std::time::Duration;

use esp_idf_svc::{
    hal::{gpio::PinDriver, i2c::I2cDriver, peripherals::Peripherals},
    timer::EspAsyncTimer,
};

use anyhow::Result;
use log::info;
use shared_bus::I2cProxy;

use crate::tpl_potentiometer::TPLPotentiometer;

static TIME_IN_SECONDS_PER_WATT: u32 = 100;
static EXPECTED_VOLAGE: f32 = 4.5;

#[derive(Clone)]
pub struct HardwareController<'a> {
    pub tpl_potentiometer: TPLPotentiometer<I2cProxy<'a, std::sync::Mutex<I2cDriver<'static>>>>,
}

impl<'a> HardwareController<'a> {
    pub async fn start_trip(
        &mut self,
        esp_async_timer: &mut EspAsyncTimer,
        energy_usage_w: u32,
    ) -> Result<()> {
        let trip_duration = energy_usage_w * TIME_IN_SECONDS_PER_WATT;
        let peripherals = Peripherals::take()?;
        let mut trip_pin = PinDriver::output(peripherals.pins.gpio4)?;
        trip_pin.set_high()?;
        info!("Motor activated for {}s", trip_duration);
        esp_async_timer
            .after(Duration::from_secs(trip_duration as u64))
            .await?;
        trip_pin.set_low()?;
        info!("Motor stopped");
        Ok(())
    }

    /// Data approximation of spec sheet: I = 901.07 / R^(0.99)
    pub async fn set_charging_speed(&mut self, charging_speed_w: u32) -> Result<()> {
        let charging_speed_a = charging_speed_w as f32 / EXPECTED_VOLAGE;
        let charging_resistance_kohm = (901.07 / charging_speed_a).log(0.99) * 1000.0;
        self.tpl_potentiometer
            .set_resistance(charging_resistance_kohm)?;
        Ok(())
    }
}
