use std::{process, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use influxdb2::RequestError;
use mqtt::AsyncClient;
use paho_mqtt as mqtt;

pub use paho_mqtt::Message;

use crate::config::MqttConfig;

#[derive(Debug)]
pub struct Error {}

impl From<mqtt::Error> for Error {
    fn from(_: mqtt::Error) -> Self {
        todo!()
    }
}

impl From<RequestError> for Error {
    fn from(_: RequestError) -> Self {
        todo!()
    }
}

#[async_trait]
pub trait MqttSubscriber {
    async fn consume<H: MqttMessageHandler + std::marker::Send + std::marker::Sync>(
        &mut self,
        handler: &H,
    ) -> Result<(), Error>;
}

pub struct MqttClientSubscriber {
    client: AsyncClient,
    config: MqttConfig,
}

impl MqttClientSubscriber {
    pub fn new(config: MqttConfig) -> Self {
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(config.host.clone())
            .client_id(config.client_id.clone())
            .finalize();

        let client = AsyncClient::new(create_opts).unwrap_or_else(|e| {
            println!("Error creating the client: {:?}", e);
            process::exit(1);
        });

        MqttClientSubscriber { client, config }
    }
}

impl Drop for MqttClientSubscriber {
    fn drop(&mut self) {
        self.client.disconnect(None);
    }
}

#[async_trait]
impl MqttSubscriber for MqttClientSubscriber {
    async fn consume<H: MqttMessageHandler + std::marker::Send + std::marker::Sync>(
        &mut self,
        handler: &H,
    ) -> Result<(), Error> {
        // Get message stream before connecting.
        let mut strm = self.client.get_stream(25);

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
        self.client.connect(conn_opts).await?;

        let subscriptions = topic_subscriptions(&self.config);

        println!("Subscribing to topics: {:?}", subscriptions);
        self.client
            .subscribe_many(
                &subscriptions,
                &subscriptions.iter().map(|_| 1i32).collect::<Vec<_>>(),
            )
            .await?;

        // Just loop on incoming messages.
        println!("Waiting for messages...");

        // release AppState lock
        // drop(st);

        // Note that we're not providing a way to cleanly shut down and
        // disconnect. Therefore, when you kill this app (with a ^C or
        // whatever) the server will get an unexpected drop and then
        // should emit the LW message.

        while let Some(msg_opt) = strm.next().await {
            // let st = state.lock().await;
            if let Some(msg) = msg_opt {
                // let points = vec![to_data_point(&msg, &st.config.mqtt)];

                let result = handler.handle(msg, &self.config);

                result.await.unwrap();

                // st.influxdb_client
                //     .write(&st.config.influxdb2.bucket, futures::stream::iter(points))
                //     .await
            } else {
                // A "None" means we were disconnected. Try to reconnect...
                println!("Lost connection. Attempting reconnect.");
                while let Err(err) = self.client.reconnect().await {
                    println!("Error reconnecting: {}", err);
                    // For tokio use: tokio::time::delay_for()
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            }
        }

        Ok(())
    }
}

fn topic_subscriptions(config: &MqttConfig) -> Vec<String> {
    config
        .topics
        .iter()
        .map(|topic| topic.name.clone())
        .collect::<Vec<_>>()
}

#[async_trait]
pub trait MqttMessageHandler {
    async fn handle(&self, msg: mqtt::Message, mqtt_config: &MqttConfig) -> Result<(), Error>;
}
