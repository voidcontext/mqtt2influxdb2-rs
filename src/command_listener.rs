use tokio::{io::AsyncReadExt, net::UnixListener, sync::mpsc::Sender};

use crate::Msg;

pub async fn listen(tx: Sender<Msg>) {
    // Bind Unix Domain Socket listener
    let listener = UnixListener::bind("/tmp/mqtt2influxdb2.sock").unwrap();

    loop {
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                println!("new client!");

                // Read from socket until EOF
                let mut buf: Vec<u8> = Vec::new();
                stream.read_to_end(&mut buf).await.unwrap();

                // Handle command received on the socket
                let command = String::from_utf8(buf).unwrap();
                match command.trim() {
                    "reload-config" => {
                        tx.send(Msg::ReloadConfig).await.unwrap();
                    }
                    "shutdown" => {
                        tx.send(Msg::Shutdown).await.unwrap();
                    }
                    str => println!("Unknown command: '{str}'"),
                }
            }
            Err(_e) => { /* connection failed */ }
        }
    }
}
