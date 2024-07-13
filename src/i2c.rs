use std::fmt::Display;
use std::time::Duration;

use embedded_hal::blocking::i2c::Write;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::i2c::{I2c, I2cConfig, I2cDriver};
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::units::Hertz;

use anyhow::Result;
use esp_idf_svc::timer::EspAsyncTimer;
use ina219::INA219;
use lazy_static::lazy_static;
use log::info;
use shared_bus::{BusManager, BusManagerSimple, I2cProxy, NullMutex};

use crate::tpl_potentiometer::TPLPotentiometer;

const POWER_INA_219_ADDRESS: u8 = 0x42;
const SOLAR_INA_219_ADDRESS: u8 = 0x40;
const TPL_ADDRESS: u8 = 0x2E;
const INA_219_MAX_EXPECTED_CURRENT: f32 = 2.0;
const INA_219_POWER_FACTOR: i16 = 20;

lazy_static! {
    static ref CURRENT_LSB: f32 = INA_219_MAX_EXPECTED_CURRENT / (2.0_f32.powf(15.0));
}

pub struct I2CDevices<'a> {
    pub power_ina_219: INA219<I2cProxy<'a, std::sync::Mutex<I2cDriver<'a>>>>,
    pub solar_ina_219: INA219<I2cProxy<'a, std::sync::Mutex<I2cDriver<'a>>>>,
    pub tpl_potentiometer: TPLPotentiometer<I2cProxy<'a, std::sync::Mutex<I2cDriver<'a>>>>,
}

impl<'a> I2CDevices<'a> {
    pub fn new(shared_bus: &'a BusManager<NullMutex<I2cDriver<'a>>>) -> Result<Self> {
        println!("Setting up I2C bus.");

        let mut power_ina_219: INA219<I2cProxy<std::sync::Mutex<I2cDriver>>> =
            INA219::new(shared_bus.acquire_i2c(), POWER_INA_219_ADDRESS);
        let mut solar_ina_219: INA219<I2cProxy<std::sync::Mutex<I2cDriver>>> =
            INA219::new(shared_bus.acquire_i2c(), SOLAR_INA_219_ADDRESS);
        let tpl_potentiometer: TPLPotentiometer<I2cProxy<std::sync::Mutex<I2cDriver>>> =
            TPLPotentiometer::new(shared_bus.acquire_i2c(), TPL_ADDRESS);

        power_ina_219.calibrate(6711)?;
        solar_ina_219.calibrate(6711)?;

        let i2c_devices = Self {
            power_ina_219,
            solar_ina_219,
            tpl_potentiometer,
        };
        Ok(i2c_devices)
    }

    pub async fn write_mqtt_messages(&mut self, esp_async_timer: &mut EspAsyncTimer) -> Result<()> {
        loop {
            info!("--- POWER INA MQTT ---");
            let power_ina_stats = build_ina_stats(&mut self.power_ina_219);
            info!("--- SOLAR INA ---");
            let solar_ina_stats = build_ina_stats(&mut self.solar_ina_219);
            esp_async_timer.after(Duration::from_millis(500)).await?;
        }
    }
}

#[derive(Debug)]
pub struct INA219Stats {
    shunt_voltage: i16,
    power: i16,
    current: f32,
    bus_voltage: u16,
}

fn build_ina_stats(ina_219: &mut INA219<I2cProxy<NullMutex<I2cDriver>>>) -> Result<INA219Stats> {
    Ok(INA219Stats {
        shunt_voltage: 10 * ina_219.shunt_voltage()?,
        power: INA_219_POWER_FACTOR * ina_219.power()?,
        current: *CURRENT_LSB * (ina_219.current()? as f32),
        bus_voltage: ina_219.voltage()?,
    })
}
