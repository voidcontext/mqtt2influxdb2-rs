use async_mutex::Mutex;
use futures::stream::StreamExt;
use influxdb2::models::DataPoint;
use mqtt::Message;
use paho_mqtt as mqtt;
use std::{process, sync::Arc, time::Duration};

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

struct AppState {
    config: Config,
    influxdb_client: influxdb2::Client,
    mqtt_client: mqtt::AsyncClient,
}

impl AppState {
    fn new(config: Config) -> Self {
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(config.mqtt.host.clone())
            .client_id(config.mqtt.client_id.clone())
            .finalize();

        let mqtt_client = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
            println!("Error creating the client: {:?}", e);
            process::exit(1);
        });

        let influxdb_client = influxdb2::Client::new(
            config.influxdb2.host.clone(),
            config.influxdb2.org.clone(),
            config.influxdb2.token.clone(),
        );
        Self {
            config,
            influxdb_client,
            mqtt_client,
        }
    }
    
    fn reload(self, new_config: Config) -> Self {
        // Unsubscribe from old topics
        self.mqtt_client.disconnect(None);
        
        drop(self.influxdb_client);

        Self::new(new_config)
    }
}

fn topic_subscriptions(config: &MqttConfig) -> Vec<String> {
    config
        .topics
        .iter()
        .map(|topic| topic.name.clone())
        .collect::<Vec<_>>()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Influxdb2

    let initial_config = Config::from_file("./config.toml").expect("Couldn't load config");

    let state = Arc::new(Mutex::new(AppState::new(initial_config)));

    // Init
    let mut st = state.lock().await;

    // Get message stream before connecting.
    let mut strm = st.mqtt_client.get_stream(25);

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
    st.mqtt_client.connect(conn_opts).await?;

    let subscriptions = topic_subscriptions(&st.config.mqtt);

    println!("Subscribing to topics: {:?}", subscriptions);
    st.mqtt_client
        .subscribe_many(
            &subscriptions,
            &subscriptions.iter().map(|_| 1i32).collect::<Vec<_>>(),
        )
        .await?;

    // Just loop on incoming messages.
    println!("Waiting for messages...");
    
    // release AppState lock
    drop(st);

    // Note that we're not providing a way to cleanly shut down and
    // disconnect. Therefore, when you kill this app (with a ^C or
    // whatever) the server will get an unexpected drop and then
    // should emit the LWT message.

    while let Some(msg_opt) = strm.next().await {
        let st = state.lock().await;
        if let Some(msg) = msg_opt {
            let points = vec![to_data_point(&msg, &st.config.mqtt)];

            st.influxdb_client
                .write(&st.config.influxdb2.bucket, futures::stream::iter(points))
                .await
                .unwrap();
        } else {
            // A "None" means we were disconnected. Try to reconnect...
            println!("Lost connection. Attempting reconnect.");
            while let Err(err) = st.mqtt_client.reconnect().await {
                println!("Error reconnecting: {}", err);
                // For tokio use: tokio::time::delay_for()
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }

    Ok(())
}
