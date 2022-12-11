use futures::stream::StreamExt;
use influxdb2::models::DataPoint;
use mqtt::Message;
use paho_mqtt as mqtt;
use std::{process, time::Duration};

use config::{Config, MqttConfig};

use serde::Deserialize;

mod config;

trait Error {}

#[derive(Debug, Deserialize)]
struct TempSensorReading {
    pub humidity: f64,
    pub pressure: f64,
    pub temperature: f64,
    pub battery: Option<i64>,
    pub linkquality: Option<i64>,
    pub voltage: Option<i64>,
}

fn to_data_point(msg: &Message, config: &MqttConfig) -> DataPoint {
    let topic = config
        .topics
        .iter()
        .find(|topic| topic.name == msg.topic())
        .expect(format!("Unexpected topic: {}", msg.topic()).as_str());

    let reading_res = serde_json::from_str::<TempSensorReading>(&msg.payload_str());
    println!("msg:{}, parsed as {:?}", msg, reading_res);

    let reading = reading_res.unwrap();

    let builder = DataPoint::builder(topic.measurement.clone())
        .field("temperature", reading.temperature)
        .field("humidity", reading.humidity)
        .field("pressure", reading.pressure);

    let builder = [
        ("battery", reading.battery),
        ("linkquality", reading.linkquality),
        ("voltage", reading.voltage),
    ]
    .iter()
    .fold(builder, |builder, (name, value)| {
        if let Some(rv) = value {
            builder.field(*name, *rv)
        } else {
            builder
        }
    });

    topic
        .tags
        .iter()
        .fold(builder, |builder, (name, value)| builder.tag(name, value))
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Influxdb2

    let config = Config::from_file("./config.toml").expect("Couldn't load config");

    let influx_client = influxdb2::Client::new(
        config.influxdb2.host,
        config.influxdb2.org,
        config.influxdb2.token,
    );
    // MQTT

    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(config.mqtt.host.clone())
        .client_id(config.mqtt.client_id.clone())
        .finalize();

    let mut cli = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
        println!("Error creating the client: {:?}", e);
        process::exit(1);
    });

    // Get message stream before connecting.
    let mut strm = cli.get_stream(25);

    // Define the set of options for the connection
    let lwt = mqtt::Message::new("test", "Async subscriber lost connection", mqtt::QOS_1);

    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(30))
        .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
        .clean_session(false)
        .will_message(lwt)
        .finalize();

    // Make the connection to the broker
    println!("Connecting to the MQTT server...");
    cli.connect(conn_opts).await?;

    let subscriptions = config
        .mqtt
        .topics
        .iter()
        .map(|topic| topic.name.clone())
        .collect::<Vec<_>>();

    println!("Subscribing to topics: {:?}", subscriptions);
    cli.subscribe_many(
        &subscriptions,
        &subscriptions.iter().map(|_| 1i32).collect::<Vec<_>>(),
    )
    .await?;

    // Just loop on incoming messages.
    println!("Waiting for messages...");

    // Note that we're not providing a way to cleanly shut down and
    // disconnect. Therefore, when you kill this app (with a ^C or
    // whatever) the server will get an unexpected drop and then
    // should emit the LWT message.

    while let Some(msg_opt) = strm.next().await {
        if let Some(msg) = msg_opt {
            let points = vec![to_data_point(&msg, &config.mqtt)];

            influx_client
                .write(&config.influxdb2.bucket, futures::stream::iter(points))
                .await
                .unwrap();
        } else {
            // A "None" means we were disconnected. Try to reconnect...
            println!("Lost connection. Attempting reconnect.");
            while let Err(err) = cli.reconnect().await {
                println!("Error reconnecting: {}", err);
                // For tokio use: tokio::time::delay_for()
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }

    Ok(())
}
