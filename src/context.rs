use std::sync::{Arc, Mutex, RwLock};

use crate::{
    car::Car, charging_controller::ChargingController, hardware_controller::HardwareController,
};

#[derive(Clone)]
pub struct Context<'a> {
    pub charging_controller_mutex: Arc<Mutex<ChargingController>>,
    pub car_rwlock: Arc<RwLock<Car>>,
    pub hardware_controller: HardwareController<'a>,
}
