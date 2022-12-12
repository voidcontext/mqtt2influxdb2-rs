use influxdb2::models::DataPoint;
use async_trait::async_trait;

use crate::{mqtt::{self, MqttMessageHandler, Error}, config::MqttConfig, TempSensorReading};

pub struct Influxdb2Writer {}

impl Influxdb2Writer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl MqttMessageHandler for Influxdb2Writer {
    async fn handle(&self, msg: mqtt::Message) -> Result<(), Error> {
        todo!()
    }
}

pub fn to_data_point(msg: &mqtt::Message, config: &MqttConfig) -> DataPoint {
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
