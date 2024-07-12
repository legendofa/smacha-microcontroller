use std::sync::{Arc, Mutex, RwLock};

use crate::{car::Car, charging_controller::ChargingController};

#[derive(Clone)]
pub struct Context {
    pub charging_controller_mutex: Arc<Mutex<ChargingController>>,
    pub car_rwlock: Arc<RwLock<Car>>,
}
