//! MQTT asynchronous client example which subscribes to an internet MQTT server and then sends
//! and receives events in its own topic.

use core::time::Duration;
use std::env;
use std::sync::{Arc, Mutex, RwLock};

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::hal::peripheral::{Peripheral, PeripheralRef};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::timer::{EspTimerService, Task};
use esp_idf_svc::wifi::*;

use anyhow::Result;
use event_service::handle_event;
use i2c::i2c_master_init;
use log::*;

mod car;
mod charging_controller;
mod context;
mod event_service;
mod handle_event_implementation;
mod handler_functions;
mod hardware_controller;
mod i2c;
mod tpl_potentiometer;

use car::Car;
use charging_controller::ChargingController;
use context::Context;

const CHANNEL: u8 = 11;

const SSID: &str = "esp-wifi-access-point";
const PASSWORD: &str = "thisismyhotspot1234";

const MQTT_URL: &str = "mqtt://192.168.71.2:1883";
const MQTT_CLIENT_ID: &str = "esp-mqtt";

const TOPICS: [&str; 4] = [
    "/charging-controller/start-charging",
    "/charging-controller/change-charging-speed",
    "/charging-controller/stop-charging",
    "/charging-controller/start-trip",
];

fn main() {
    if let Err(e) = run_main() {
        eprintln!("Application error: {:?}", e);
    }
}

fn run_main() -> Result<()> {
    {
        env::set_var("RUST_BACKTRACE", "1");
        esp_idf_svc::sys::link_patches();
        esp_idf_svc::log::EspLogger::initialize_default();

        let peripherals = Peripherals::take()?;

        let i2c_master = i2c_master_init(
            peripherals.i2c0.into_ref(),
            peripherals.pins.gpio21.into(),
            peripherals.pins.gpio22.into(),
            100000.into(),
        )?;

        let shared_bus: &'static _ = shared_bus::new_std!(I2cDriver = i2c_master)
            .expect("Shared bus could not be initialized");

        esp_idf_svc::hal::task::block_on(async {
            let _wifi = wifi_create(peripherals.modem.into_ref())?;
            info!("Wifi created");

            let (mut client, mut conn) = mqtt_create(MQTT_URL, MQTT_CLIENT_ID)?;
            info!("MQTT client created");

            //let mut i2c_devices = I2CDevices::new(&shared_bus.acquire_i2c()).unwrap();

            //let hardware_controller = HardwareController {
            //    tpl_potentiometer: i2c_devices.tpl_potentiometer.clone(),
            //};

            run(&mut client, &mut conn).await?;
            Ok::<(), anyhow::Error>(())
        })
    }
}

async fn run(
    client: &mut EspMqttClient<'_>,
    connection: &mut EspMqttConnection,
    //hardware_controller: HardwareController<'_>,
) -> Result<()> {
    std::thread::scope(|s| {
        info!("About to start the MQTT client");

        let context = initialize_context()?;

        std::thread::Builder::new()
            .stack_size(6000)
            .spawn_scoped(s, move || {
                info!("MQTT Listening for messages");

                while let Ok(event) = connection.next() {
                    match handle_event(event.payload(), context.clone()) {
                        Ok(_) => (),
                        Err(error) => info!("{error}"),
                    }
                }

                info!("Connection closed");
            })?;

        // Using `pin!` is optional, but it optimizes the memory size of the Futures
        for topic in TOPICS {
            while let Err(e) = client.subscribe(topic, QoS::AtMostOnce) {
                error!("Failed to subscribe to topic \"{topic}\": {e}, retrying...");

                // Re-try in 0.5s
                std::thread::sleep(Duration::from_millis(500));

                continue;
            }
        }

        loop {
            // Just to give a chance of our connection to get even the first published message
            std::thread::sleep(Duration::from_millis(500));

            let payload = "Hello from esp-mqtt-demo!";

            /* loop {
                client
                    .publish(MQTT_TOPIC, QoS::AtMostOnce, false, payload.as_bytes())
                    .await?;

                info!("Published \"{payload}\" to topic");

                let sleep_secs = 2;

                info!("Now sleeping for {sleep_secs}s...");
                second_timer.after(Duration::from_secs(sleep_secs)).await?;
            } */
        }
    })
}

fn mqtt_create(
    url: &str,
    client_id: &str,
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(
        url,
        &MqttClientConfiguration {
            client_id: Some(client_id),
            ..Default::default()
        },
    )?;

    Ok((mqtt_client, mqtt_conn))
}

fn wifi_create(modem: PeripheralRef<'static, Modem>) -> Result<EspWifi, EspError> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut esp_wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sys_loop)?;

    let wifi_configuration = Configuration::AccessPoint(AccessPointConfiguration {
        ssid: SSID.try_into().unwrap(),
        ssid_hidden: false,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: CHANNEL,
        ..Default::default()
    });
    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    info!(
        "Created Wi-Fi with WIFI_SSID `{}` and WIFI_PASS `{}`",
        SSID, PASSWORD
    );

    Ok(esp_wifi)
}

fn initialize_context() -> Result<Context> {
    let context = Context {
        charging_controller_mutex: Arc::new(Mutex::new(ChargingController::new())),
        car_rwlock: Arc::new(RwLock::new(Car::new(3700, 0, 100)?)),
    };
    {
        let mut charging_controller = context.charging_controller_mutex.lock().unwrap();
        charging_controller.connect_car(context.car_rwlock.clone())?;
    };
    Ok(context)
}
