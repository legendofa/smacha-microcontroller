use std::{
    io::{Error, ErrorKind},
    time::Duration,
};

use anyhow::Result;
use esp_idf_svc::{
    hal::{gpio::PinDriver, peripherals::Peripherals},
    timer::EspAsyncTimer,
};
use log::info;

/// Defines when the battery is counted as full.
static FULL_CAPACITY_MARGIN: u32 = 10;

#[derive(Clone, Copy)]
pub struct Car {
    charging_capacity_wh: u32,
    current_charge_wh: u32,
    pub max_charging_speed_w: u32,
}

impl Car {
    pub fn new(
        charging_capacity_wh: u32,
        current_charge_wh: u32,
        max_charging_speed_w: u32,
    ) -> Result<Self> {
        if current_charge_wh > charging_capacity_wh {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "Current charge cannot exceed charging capacity",
            ))?
        } else {
            Ok(Car {
                charging_capacity_wh,
                current_charge_wh,
                max_charging_speed_w,
            })
        }
    }

    pub fn is_fully_charged(&self) -> bool {
        self.current_charge_wh >= self.charging_capacity_wh - FULL_CAPACITY_MARGIN
    }

    pub fn change_current_charge(&mut self, new_charge_wh: u32) -> Result<()> {
        if new_charge_wh > self.charging_capacity_wh {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "New charge cannot exceed charging capacity",
            ))?
        } else {
            self.current_charge_wh = new_charge_wh;
        }
        Ok(())
    }
}
