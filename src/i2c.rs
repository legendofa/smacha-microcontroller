use std::time::Duration;

use esp_idf_svc::hal::delay::BLOCK;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::i2c::{I2c, I2cConfig, I2cDriver};
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::units::Hertz;

use anyhow::Result;
use esp_idf_svc::timer::EspAsyncTimer;
use ina219::INA219;
use log::info;

const INA_219_ADDRESS: u8 = 0x42;
const TPL_ADDRESS: u8 = 0x2E;

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

    i2c_master.write(TPL_ADDRESS, &[0x7F], BLOCK)?;

    let mut ina_219 = INA219::new(i2c_master, INA_219_ADDRESS);

    ina_219.calibrate(6711)?;
    loop {
        info!("Shunt voltage: {}uV", 10 * ina_219.shunt_voltage()?);
        info!("Power: {}", ina_219.power()?);
        info!("Current: {}", 0.00006103 * (ina_219.current()? as f32));
        info!("Bus voltage: {}", ina_219.voltage()?);
        esp_async_timer.after(Duration::from_millis(500)).await?;
    }
    Ok(())
}
