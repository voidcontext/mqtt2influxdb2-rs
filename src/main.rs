use service::Service;

use config::Config;

use serde::Deserialize;

mod command_listener;
mod config;
mod influxdb;
mod mqtt;
mod service;

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

    let mut service = Service::new(initial_config);

    service.start().await;

    // let state = Arc::new(Mutex::new(&initial_state));
    // let cmd_listener_state = Arc::clone(&state);
    // tokio::spawn(async move {

    // });

    Ok(())
}
