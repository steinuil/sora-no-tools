use serde_json::json;
use std::io;
use structopt::StructOpt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

#[cfg(windows)]
struct Kopipe(NamedPipeClient);

#[cfg(unix)]
use tokio::net::UnixStream;

#[cfg(unix)]
struct Kopipe(UnixStream);

impl Kopipe {
    #[cfg(windows)]
    async fn open(path: &str) -> io::Result<Self> {
        Ok(Kopipe(ClientOptions::new().open(path)?))
    }

    #[cfg(unix)]
    async fn open(path: &str) -> io::Result<Self> {
        Ok(Kopipe(UnixStream::connect(path).await?))
    }

    async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf).await
    }

    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf).await
    }
}

#[derive(StructOpt)]
struct Options {
    #[structopt(long)]
    pub server: String,

    #[structopt(long)]
    pub mpv_socket: String,
}

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let opts = Options::from_args();

    let mut messages_resp = reqwest::get(&opts.server)
        .await
        .expect("Could not connect to the server");
    log::info!("Connected to the server [address = {}]", opts.server);

    let mut pipe = Kopipe::open(&opts.mpv_socket)
        .await
        .expect("Could not connect to mpv");
    log::info!(
        "Connected to the mpv IPC socket [path = {}]",
        opts.mpv_socket
    );

    let mut read_buf = [0; 128];
    loop {
        tokio::select! {
            chunk = messages_resp.chunk() => {
                match chunk {
                    Ok(Some(chunk)) => {
                        if let Ok(message) = String::from_utf8(chunk.to_vec()) {
                            let cmd = json!({
                                "command": [
                                    "script-message-to",
                                    "coof",
                                    "danmaku-message",
                                    &message
                                ]
                            });

                            pipe.write(&[serde_json::to_vec(&cmd).unwrap().as_slice(), b"\n"].concat())
                                .await
                                .expect("mpv IPC socket closed");

                            log::debug!("Sent message: {}", message);
                        }
                    }
                    _ => break
                }
            }

            read = pipe.read(&mut read_buf) => {
                match read {
                    Ok(0) => break,
                    Ok(read) => {
                        let _ = std::str::from_utf8(&read_buf[..read])
                            .map(|cmd| log::debug!("Received message: {}", cmd.trim()));
                    }
                    _ => break
                }
            }
        }
    }
}
