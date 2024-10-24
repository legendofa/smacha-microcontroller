//! MQTT asynchronous client example which subscribes to an internet MQTT server and then sends
//! and receives events in its own topic.

use core::pin::pin;
use core::time::Duration;
use std::sync::{Arc, Mutex, RwLock};

use car::Car;
use charging_controller::ChargingController;
use context::Context;
use embassy_futures::select::{select, Either};

mod car;
mod charging_controller;
mod context;
mod event_service;
mod handle_event_implementation;
mod handler_functions;
mod i2c;
mod tpl_potentiometer;

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
use i2c::{i2c_master_init, I2CDevices};
use log::*;

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
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let timer_service = EspTimerService::new().unwrap();

    let peripherals = Peripherals::take().unwrap();

    let i2c_master = i2c_master_init(
        peripherals.i2c0.into_ref(),
        peripherals.pins.gpio21.into(),
        peripherals.pins.gpio22.into(),
        100000.into(),
    )
    .unwrap();

    let shared_bus: &'static _ = shared_bus::new_std!(I2cDriver = i2c_master).unwrap();

    esp_idf_svc::hal::task::block_on(async {
        let _wifi = wifi_create(peripherals.modem.into_ref())?;
        info!("Wifi created");

        let mut i2c_devices = I2CDevices::new(&shared_bus.acquire_i2c()).unwrap();

        let (mut client, mut conn) = mqtt_create(MQTT_URL, MQTT_CLIENT_ID)?;
        info!("MQTT client created");

        run(&mut client, &mut conn, &timer_service, &mut i2c_devices).await?;
        Ok::<(), anyhow::Error>(())
    })
    .unwrap();
}

async fn run(
    client: &mut EspAsyncMqttClient,
    connection: &mut EspAsyncMqttConnection,
    timer_service: &EspTimerService<Task>,
    i2c_devices: &mut I2CDevices<'_>,
) -> Result<()> {
    info!("About to start the MQTT client");

    let mut first_timer = timer_service.timer_async()?;
    let mut second_timer = timer_service.timer_async()?;

    let context = initialize_context()?;

    let res = select(
        // Need to immediately start pumping the connection for messages, or else subscribe() and publish() below will not work
        // Note that when using the alternative structure and the alternative constructor - `EspMqttClient::new_cb` - you don't need to
        // spawn a new thread, as the messages will be pumped with a backpressure into the callback you provide.
        // Yet, you still need to efficiently process each message in the callback without blocking for too long.
        //
        // Note also that if you go to http://tools.emqx.io/ and then connect and send a message to topic
        // "esp-mqtt-demo", the client configured here should receive it.
        pin!(async move {
            info!("MQTT Listening for messages");

            while let Ok(event) = connection.next().await {
                match handle_event(&mut first_timer, event.payload(), context.clone()).await {
                    Ok(_) => (),
                    Err(error) => info!("{error}"),
                }
            }

            info!("Connection closed");

            Ok(())
        }),
        pin!(async move {
            // Using `pin!` is optional, but it optimizes the memory size of the Futures
            for topic in TOPICS {
                while let Err(e) = client.subscribe(topic, QoS::AtMostOnce).await {
                    error!("Failed to subscribe to topic \"{topic}\": {e}, retrying...");

                    // Re-try in 0.5s
                    second_timer.after(Duration::from_millis(500)).await?;

                    continue;
                }
            }

            // Just to give a chance of our connection to get even the first published message
            second_timer.after(Duration::from_millis(500)).await?;

            loop {
                i2c_devices
                    .write_mqtt_messages(&mut second_timer, client)
                    .await
                    .unwrap();
            }
        }),
    )
    .await;

    match res {
        Either::First(res) => res,
        Either::Second(res) => res,
    }
}

fn mqtt_create(
    url: &str,
    client_id: &str,
) -> Result<(EspAsyncMqttClient, EspAsyncMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspAsyncMqttClient::new(
        url,
        &MqttClientConfiguration {
            client_id: Some(client_id),
            ..Default::default()
        },
    )?;

    Ok((mqtt_client, mqtt_conn))
}

fn wifi_create(modem: PeripheralRef<'static, Modem>) -> Result<EspWifi<'static>, EspError> {
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
