use serde::Deserialize;
use std::{collections::HashMap, fs};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub influxdb2: Influxdb,
    pub mqtt: Mqtt,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Influxdb {
    pub host: String,
    pub org: String,
    pub token: String,
    pub bucket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Mqtt {
    pub host: String,
    pub client_id: String,
    pub topics: Vec<Topic>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Topic {
    pub name: String,
    pub measurement: String,
    pub tags: HashMap<String, String>,
}

impl Config {
    pub fn from_file(file_name: &str) -> Result<Config, toml::de::Error> {
        let config_str = fs::read_to_string(file_name)
            .unwrap_or_else(|_| panic!("Cannot found file: {}", file_name));

        toml::from_str(&config_str)
    }
}
