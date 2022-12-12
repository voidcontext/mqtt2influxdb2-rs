use async_mutex::Mutex;
use async_trait::async_trait;
use futures::stream::StreamExt;
use influxdb::Influxdb2Writer;
use influxdb2::models::DataPoint;
use mqtt::{MqttClientSubscriber, MqttSubscriber};
use std::{sync::Arc, time::Duration};

use config::{Config, MqttConfig};

use serde::Deserialize;

use crate::app_state::AppState;

mod app_state;
mod command_listener;
mod config;
mod influxdb;
mod mqtt;

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


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Influxdb2

    let initial_config = Config::from_file("./config.toml").expect("Couldn't load config");

    let state = Arc::new(Mutex::new(AppState::new(initial_config)));
    
    let handler = Influxdb2Writer::new();
    
    let subscriber = MqttClientSubscriber::new();
    subscriber.consume(handler);
    
    
    // let cmd_listener_state = Arc::clone(&state);
    // tokio::spawn(async move {
        
    // });

    // Init
    // let mut st = state.lock().await;


    Ok(())
}
