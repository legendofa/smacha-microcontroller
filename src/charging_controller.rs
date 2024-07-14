use anyhow::Result;
use std::{
    io::{Error, ErrorKind},
    sync::{Arc, RwLock},
};

use crate::car::Car;

pub enum ChargingController {
    Disconnected,
    Connected {
        car_rwlock: Arc<RwLock<Car>>,
    },
    Charging {
        car_rwlock: Arc<RwLock<Car>>,
        charging_speed_w: u32,
    },
}

impl ChargingController {
    pub fn new() -> Self {
        ChargingController::Disconnected
    }

    pub fn connect_car(&mut self, car_rwlock: Arc<RwLock<Car>>) -> Result<()> {
        match self {
            ChargingController::Disconnected => {
                *self = ChargingController::Connected { car_rwlock }
            }
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Car is already connected",
            ))?,
        }
        Ok(())
    }

    pub fn disconnect_car(&mut self) -> Result<()> {
        match self {
            ChargingController::Connected { car_rwlock: _ } => {
                *self = ChargingController::Disconnected
            }
            ChargingController::Charging { .. } => Err(Error::new(
                ErrorKind::InvalidInput,
                "Cannot disconnect while charging",
            ))?,
            ChargingController::Disconnected => {
                Err(Error::new(ErrorKind::InvalidInput, "No car connected"))?
            }
        };
        Ok(())
    }

    pub fn start_charging(&mut self, charging_speed_w: u32) -> Result<()> {
        match self {
            ChargingController::Connected { car_rwlock } => {
                {
                    let car = car_rwlock.read().unwrap();
                    if charging_speed_w > car.max_charging_speed_w {
                        Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Charging speed exceeds car's maximum charging speed",
                        ))?
                    } else if car.is_fully_charged() {
                        Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Car is already fully charged",
                        ))?
                    }
                }
                *self = ChargingController::Charging {
                    car_rwlock: car_rwlock.clone(),
                    charging_speed_w,
                };
            }
            ChargingController::Charging { .. } => {
                Err(Error::new(ErrorKind::InvalidInput, "Already charging"))?
            }
            ChargingController::Disconnected => {
                Err(Error::new(ErrorKind::InvalidInput, "No car connected"))?
            }
        }
        Ok(())
    }

    pub fn change_charging_speed(&mut self, new_charging_speed_w: u32) -> Result<()> {
        match self {
            ChargingController::Charging {
                car_rwlock,
                charging_speed_w: _,
            } => {
                if new_charging_speed_w > car_rwlock.read().unwrap().max_charging_speed_w {
                    Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Charging speed exceeds car's maximum charging speed",
                    ))?
                } else {
                    *self = ChargingController::Charging {
                        car_rwlock: car_rwlock.clone(),
                        charging_speed_w: new_charging_speed_w,
                    };
                }
                Ok(())
            }
            _ => panic!("Not currently charging"),
        }
    }

    pub fn stop_charging(&mut self) -> Result<()> {
        match self {
            ChargingController::Charging { car_rwlock, .. } => {
                *self = ChargingController::Connected {
                    car_rwlock: car_rwlock.clone(),
                }
            }
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Not currently charging",
            ))?,
        }
        Ok(())
    }
}
