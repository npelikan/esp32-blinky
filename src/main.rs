use anyhow::Result;
use embedded_hal::delay::DelayNs;
use embedded_svc::mqtt::client::{
    EventPayload::Error, EventPayload::Received, QoS,
};
use esp_idf_hal::gpio::PinDriver;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        delay,
        prelude::*
        },
    mqtt::client::{EspMqttClient, MqttClientConfiguration},
    nvs::EspDefaultNvsPartition,
};
use esp_idf_sys::EspError;
use log::{error, info, warn};
use esp_idf_svc::wifi::*;
use std::{thread::sleep, time::Duration};

const UUID: &str = "esp32-dev";

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

const MQTT_URL: &str = "mqtt://192.168.1.174:1883";
const MQTT_CLIENT_ID: &str = "esp-mqtt";
const MQTT_TOPIC: &str = "esp/led";


fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take().unwrap();

    // Connect to the Wi-Fi network
    // let _wifi: Result<EspWifi, EspError> = wifi_create(
    //     SSID,
    //     PASSWORD,
    //     &sysloop,
    //     &nvs,
    // );

    info!("Connecting to WiFi");

    let mut esp_wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs.clone()))?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop.clone())?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        ..Default::default()
    }))?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    info!("Our UUID is:");
    info!("{}", UUID);

    let pins = peripherals.pins;
    let mut delay = delay::Ets;

    delay.delay_ms(3000);

    let mqtt_config = MqttClientConfiguration {
        client_id : Some(MQTT_CLIENT_ID),
        ..Default::default()
    };

    let mut led = PinDriver::output(pins.gpio2).unwrap();

    // 1. Create a client with default configuration and empty handler
    // ANCHOR: mqtt_client
    let mut client =
        EspMqttClient::new_cb(
            &MQTT_URL,
            &mqtt_config,
            move |message_event| match message_event.payload() {
                Received { data, .. } => {
                    info!("{:?}", data);
                    let parsed_string = String::from_utf8(data.to_vec());
                    match parsed_string.expect("Unexpected Failure").to_uppercase().as_str() {
                        msg if msg.contains("ON") => {
                            info!("Turning LED ON");
                            led.set_high().unwrap();
                        }
                        msg if msg.contains("OFF") => {
                            info!("Turning LED OFF"); 
                            led.set_low().unwrap();
                        }
                        _ => info!("Unknown command!"),
                    }
                },
                Error(e) => warn!("Received error from MQTT: {:?}", e),
                _ => info!("Received from MQTT: {:?}", message_event.payload()),
            },
        )?;
    // ANCHOR_END: mqtt_client

    // 2. publish an empty hello message
    let payload: &[u8] = &[];
    client.publish(MQTT_TOPIC, QoS::AtLeastOnce, true, payload)?;

    client.subscribe(MQTT_TOPIC, QoS::AtLeastOnce)?;

    loop {
        sleep(Duration::from_secs(1));
    }
}
