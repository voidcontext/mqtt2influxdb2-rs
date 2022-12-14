use std::env;

use futures::future::AbortHandle;

use config::Config;

use serde::Deserialize;
use tokio::sync::mpsc::{self, Sender};

use crate::{
    influxdb::Influxdb2Writer,
    mqtt::{MqttClientSubscriber, MqttSubscriber},
};

use futures::stream::StreamExt;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;

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

fn spawn_consumer(config: Config) -> AbortHandle {
    println!("Spawning consumer...");
    let (abort_handle, abort_registration) = AbortHandle::new_pair();

    let mut mqtt_subscriber = MqttClientSubscriber::new(config.mqtt.clone());
    let mqtt_message_handler = Influxdb2Writer::new(config.influxdb2);

    tokio::spawn(async move {
        mqtt_subscriber
            .consume(&mqtt_message_handler, abort_registration)
            .await
            .unwrap();
    });

    println!("Consumer started.");

    abort_handle
}

async fn signal_handler(mut signals: Signals, tx: Sender<Msg>) {
    while let Some(signal) = signals.next().await {
        match signal {
            SIGHUP => {
                // Reload configuration
                // Reopen the log file
                tx.send(Msg::ReloadConfig).await.unwrap();
            }
            SIGTERM | SIGINT | SIGQUIT => {
                tx.send(Msg::Shutdown).await.unwrap();
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum Msg {
    ReloadConfig,
    Shutdown,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel::<Msg>(16);

    // Start the inital instance of the consumer
    let config_file = env::var("MQTT2INFLUXDB2_CONFIG_FILE")
        .expect("The env var MQTT2INFLUXDB2_CONFIG_FILE must be set");
    let initial_config = Config::from_file(&config_file).expect("Couldn't load config");
    let mut abort_handle = spawn_consumer(initial_config);

    // Set up signal handler
    let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;

    let handle = signals.handle();
    let signals_task = tokio::spawn(signal_handler(signals, tx.clone()));

    // unix listener
    let cmd_listener_task = tokio::spawn(command_listener::listen(tx.clone()));

    // main loop
    while let Some(msg) = rx.recv().await {
        match msg {
            Msg::ReloadConfig => {
                // Abort the current consumer stream
                abort_handle.abort();

                // Re-read the config file
                let config = Config::from_file(&config_file).expect("Couldn't load config");

                println!("new config: {:?}", config);

                // Spwawn a new consumer, and update the abort handle
                abort_handle = spawn_consumer(config);
            }
            Msg::Shutdown => {
                // stop mqtt consumer stream
                abort_handle.abort();

                // stop signal handler
                handle.close();
                signals_task.abort();

                //  stop unix socket listener
                cmd_listener_task.abort();
                // remove socket
                tokio::fs::remove_file("/tmp/mqtt2influxdb2.sock")
                    .await
                    .unwrap();

                // terminate main thread
                rx.close();
            }
        }
    }

    Ok(())
}
