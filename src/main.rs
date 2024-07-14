//! MQTT blocking client example which subscribes to an internet MQTT server and then sends
//! and receives events in its own topic.
#![feature(const_fn_floating_point_arithmetic)]

use anyhow::Result;
use core::pin::pin;
use core::time::Duration;
use esp_idf_svc::hal::i2c::I2cDriver;
use event_service::handle_event;
use handle_event_implementation::handle_event_implementation;
use i2c::{i2c_master_init, I2CDevices};
use std::sync::{Arc, Mutex, RwLock};

use car::Car;
use charging_controller::ChargingController;
use context::Context;
use embassy_futures::select::{select3, Either3};

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::timer::{EspTimerService, Task};
use esp_idf_svc::wifi::*;

mod car;
mod charging_controller;
mod context;
mod event_service;
mod handle_event_implementation;
mod handler_functions;
mod i2c;
mod tpl_potentiometer;

use log::*;
const CHANNEL: u8 = 11;

const SSID: &str = "esp-wifi-access-point";
const PASSWORD: &str = "thisismyhotspot1234";

const MQTT_URL: &str = "mqtt://192.168.71.2:1883";
const MQTT_CLIENT_ID: &str = "esp-mqtt";

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let timer_service = EspTimerService::new()?;

    let peripherals = Peripherals::take()?;

    let i2c_master = i2c_master_init(
        peripherals.i2c0,
        peripherals.pins.gpio21.into(),
        peripherals.pins.gpio22.into(),
        100000.into(),
    )?;

    let shared_bus: &'static _ = shared_bus::new_std!(I2cDriver = i2c_master).unwrap();

    esp_idf_svc::hal::task::block_on(async {
        let mut timer = timer_service.timer_async().unwrap();

        let mut i2c_devices = I2CDevices::new(&shared_bus.acquire_i2c()).unwrap();

        let _wifi = create_wifi().unwrap();
        info!("Wifi created");

        /* let (mut client, conn) = mqtt_create(MQTT_URL, MQTT_CLIENT_ID).unwrap();
        info!("MQTT client created");

        for topic in ["/wall-plug/stats", "/solar-panel/stats"] {
            while let Err(e) = client.subscribe(topic, QoS::AtMostOnce).await {
                error!("Failed to subscribe to topic \"{topic}\": {e}, retrying...");

                // Re-try in 0.5s
                timer.after(Duration::from_millis(500)).await.unwrap();

                continue;
            }
        }

        i2c_devices
            .write_mqtt_messages(&mut timer, &mut client)
            .await
            .unwrap(); */

        async move { loop {} }.await;

        //run(&mut client, &mut conn, timer_service).await
    });
    Ok(())
}

async fn run(
    mqtt_client: &mut EspAsyncMqttClient,
    connection: &mut EspAsyncMqttConnection,
    timer_service: EspTimerService<Task>,
) -> Result<()> {
    info!("About to start the MQTT client");

    let context = Context {
        charging_controller_mutex: Arc::new(Mutex::new(ChargingController::new())),
        car_rwlock: Arc::new(RwLock::new(Car::new(3700, 0, 100)?)),
    };

    let topics: Vec<&str> = vec![
        "/charging-controller/start-charging",
        "/charging-controller/change-charging-speed",
        "/charging-controller/stop-charging",
        "/charging-controller/start-trip",
    ];

    let mut first_timer = timer_service.timer_async()?;
    let mut second_timer = timer_service.timer_async()?;
    let third_timer = timer_service.timer_async()?;

    let res = select3(
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
                handle_event(&mut first_timer, event.payload(), context.clone()).await?;
            }

            info!("Connection closed");

            Ok(())
        }),
        pin!(async move {
            // Using `pin!` is optional, but it optimizes the memory size of the Futures
            for topic in topics {
                while let Err(e) = mqtt_client.subscribe(topic, QoS::AtMostOnce).await {
                    error!("Failed to subscribe to topic \"{topic}\": {e}, retrying...");

                    // Re-try in 0.5s
                    second_timer.after(Duration::from_millis(500)).await?;

                    continue;
                }
            }

            // Just to give a chance of our connection to get even the first published message
            second_timer.after(Duration::from_millis(500)).await?;

            let payload = "Hello from esp-mqtt-demo!";

            mqtt_client
                .publish(
                    "/charging-controller/start-charging",
                    QoS::AtMostOnce,
                    false,
                    payload.as_bytes(),
                )
                .await?;

            info!("Published \"{payload}\" to topic");

            let sleep_secs = 2;

            info!("Now sleeping for {sleep_secs}s...");
            second_timer.after(Duration::from_secs(sleep_secs)).await?;
            Ok(())
        }),
        pin!(async move { Ok(()) }),
    )
    .await;

    match res {
        Either3::First(res) => res,
        Either3::Second(res) => res,
        Either3::Third(res) => res,
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

fn create_wifi() -> Result<EspWifi<'static>, EspError> {
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut esp_wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;
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
