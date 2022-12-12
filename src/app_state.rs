use std::process;
use paho_mqtt as mqtt;

use crate::config::Config;

pub struct AppState {
    pub config: Config,
    pub influxdb_client: influxdb2::Client,
    pub mqtt_client: mqtt::AsyncClient,
}

impl AppState {
    pub fn new(config: Config) -> Self {
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
    
    pub fn reload(self, new_config: Config) -> Self {
        // Unsubscribe from old topics
        self.mqtt_client.disconnect(None);
        
        drop(self.influxdb_client);

        Self::new(new_config)
    }
}
