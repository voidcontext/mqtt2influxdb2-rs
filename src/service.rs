use crate::{
    config::Config,
    influxdb::Influxdb2Writer,
    mqtt::{MqttClientSubscriber, MqttSubscriber},
};

pub struct Service {
    mqtt_subscriber: MqttClientSubscriber,
    mqtt_message_handler: Influxdb2Writer,
}

impl Service {
    pub fn new(config: Config) -> Self {
        Self {
            mqtt_subscriber: MqttClientSubscriber::new(config.mqtt),
            mqtt_message_handler: Influxdb2Writer::new(config.influxdb2),
        }
    }

    pub async fn start(&mut self) {
        self.mqtt_subscriber
            .consume(&self.mqtt_message_handler)
            .await
            .unwrap();
    }

    pub fn reload(self, new_config: Config) -> Self {
        drop(self);

        let service = Self::new(new_config);
        service
    }
}
