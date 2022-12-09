use std::fs;
use serde::Deserialize;


#[derive(Deserialize)]
pub struct Config {
	pub influxdb2: Influxdb2Config,
	pub mqtt: MqttConfig	
}

#[derive(Deserialize)]
pub struct Influxdb2Config {
    pub host: String,
    pub org: String,
    pub token: String,
	pub bucket: String
}

#[derive(Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub topics: Vec<String>
}

impl Config {
	pub fn from_file(file_name: &str) -> Result<Config, toml::de::Error> {
        let config_str = fs::read_to_string(file_name)
            .unwrap_or_else(|_| panic!("Cannot found file: {}", file_name));

        toml::from_str(&config_str)
    }
}
