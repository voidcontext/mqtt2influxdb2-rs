use async_trait::async_trait;
use influxdb2::{models::DataPoint, Client};

use crate::{
    config::{Influxdb, Mqtt},
    mqtt::{self, Error, MessageHandler},
    TempSensorReading,
};

pub struct Writer {
    config: Influxdb,
    client: Client,
}

impl Writer {
    pub fn new(config: Influxdb) -> Self {
        let client = influxdb2::Client::new(
            config.host.clone(),
            config.org.clone(),
            config.token.clone(),
        );
        Self { config, client }
    }
}

#[async_trait]
impl MessageHandler for Writer {
    async fn handle(&self, msg: mqtt::Message, mqtt_config: &Mqtt) -> Result<(), Error> {
        self.client
            .write(
                &self.config.bucket,
                futures::stream::iter([to_data_point(&msg, mqtt_config)]),
            )
            .await
            .map_err(Error::from)
    }
}

fn to_data_point(msg: &mqtt::Message, config: &Mqtt) -> DataPoint {
    let topic = config
        .topics
        .iter()
        .find(|topic| topic.name == msg.topic())
        .unwrap_or_else(|| panic!("Unexpected topic: {}", msg.topic()));

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
