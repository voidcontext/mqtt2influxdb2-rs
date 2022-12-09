use influxdb2::models::{data_point::DataPointBuilder, DataPoint};
use paho_mqtt as mqtt;
use std::{process, time::Duration};
use futures::stream::StreamExt;

use config::Config;

use serde::Deserialize;

mod config;


trait Error {}

#[derive(Debug, Deserialize)]
struct TempSensorReading {
    pub humidity: f64,
    pub pressure: f64,
    pub temperature: f64,
    pub battery: Option<u64>,
    pub linkquality: Option<u64>,
    pub voltage: Option<u64>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Influxdb2
    
    let config = Config::from_file("./config.toml").expect("Couldn't load config");
    
    let influx_client = influxdb2::Client::new(config.influxdb2.host, config.influxdb2.org, config.influxdb2.token);

    
    // MQTT
    
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(config.mqtt.host)
        .client_id("rust_async_subscribe")
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

    println!("Subscribing to topics: {:?}", config.mqtt.topics);
    cli.subscribe_many(&config.mqtt.topics, &[1]).await?;

    // Just loop on incoming messages.
    println!("Waiting for messages...");

    // Note that we're not providing a way to cleanly shut down and
    // disconnect. Therefore, when you kill this app (with a ^C or
    // whatever) the server will get an unexpected drop and then
    // should emit the LWT message.

    while let Some(msg_opt) = strm.next().await {
        if let Some(msg) = msg_opt {
            let reading_res = serde_json::from_str::<TempSensorReading>(&msg.payload_str());
            println!("msg:{}, parsed as {:?}", msg,reading_res);
            
            let reading = reading_res.unwrap();
            
            let points = vec![
                DataPoint::builder("sensor-reading")
                    .tag("location", "gabor-office")
                    .tag("sensor", "temperature")
                    .field("temperature", reading.temperature)
                    .build()
                    .unwrap()
            ];
            
            influx_client.write(&config.influxdb2.bucket, futures::stream::iter(points)).await.unwrap();
        }
        else {
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
