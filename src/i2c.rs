use std::time::Duration;

use esp_idf_svc::hal::delay::BLOCK;
use esp_idf_svc::hal::gpio::{AnyIOPin, InputPin, OutputPin};
use esp_idf_svc::hal::i2c::{I2c, I2cConfig, I2cDriver, I2cSlaveConfig, I2cSlaveDriver};
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::units::Hertz;

use anyhow::Result;
use esp_idf_svc::timer::EspAsyncTimer;
use ina219::INA219;
use log::info;

const SLAVE_ADDR: u8 = 0x22;

fn i2c_master_init<'d>(
    i2c: impl Peripheral<P = impl I2c> + 'd,
    sda: AnyIOPin,
    scl: AnyIOPin,
    baudrate: Hertz,
) -> anyhow::Result<I2cDriver<'d>> {
    let config = I2cConfig::new().baudrate(baudrate);
    let driver = I2cDriver::new(i2c, sda, scl, &config)?;
    Ok(driver)
}

pub async fn i2c_test(esp_async_timer: &mut EspAsyncTimer) -> Result<()> {
    println!("Starting I2C self test");

    let peripherals = Peripherals::take()?;

    let mut i2c_master = i2c_master_init(
        peripherals.i2c0,
        peripherals.pins.gpio21.into(),
        peripherals.pins.gpio22.into(),
        100.kHz().into(),
    )?;

    let mut ina_219 = INA219::new(i2c_master, SLAVE_ADDR);
    ina_219.calibrate(6711)?;
    loop {
        info!("Shunt voltage: {}mV", ina_219.shunt_voltage()?);
        info!("Power: {}mW", ina_219.power()?);
        info!("Current: {}mA", ina_219.current()?);
        info!("Bus voltage: {}V", ina_219.voltage()?);
        esp_async_timer.after(Duration::from_millis(500)).await?;
    }
    Ok(())
}
