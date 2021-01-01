use serde_json::json;
use std::{
    env,
    io::{Result as IoResult, Write},
    path::Path,
    process::exit,
};

#[cfg(windows)]
use miow::pipe::NamedPipe;
#[cfg(windows)]
use std::{
    fs::OpenOptions,
    os::windows::{
        fs::OpenOptionsExt,
        io::{FromRawHandle, IntoRawHandle},
    },
};

#[cfg(unix)]
use std::os::unix::net::UnixStream;

#[cfg(windows)]
struct Kopipe(NamedPipe);

#[cfg(unix)]
struct Kopipe(UnixStream);

impl Kopipe {
    #[cfg(windows)]
    fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let mut opts = OpenOptions::new();
        opts.read(false).write(true).custom_flags(0x40000000);
        let file = opts.open(path)?;
        Ok(Kopipe(unsafe {
            NamedPipe::from_raw_handle(file.into_raw_handle())
        }))
    }

    #[cfg(unix)]
    fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        Ok(Kopipe(UnixStream::connect(path)?))
    }

    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.0.write(buf)
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <server> <mpv_socket>", args[0]);
        exit(1);
    }

    let mut messages_resp = reqwest::get(&args[1])
        .await
        .map_err(|_| "Could not connect to the server")
        .unwrap();
    println!("Connected to the server on {}", args[1]);

    let mut pipe = Kopipe::open(&args[2])
        .map_err(|_| "Could not connect to mpv")
        .unwrap();
    println!("Connected to the mpv IPC socket on {}", args[2]);

    while let Some(chunk) = messages_resp.chunk().await.unwrap() {
        if let Ok(message) = String::from_utf8(chunk.to_vec()) {
            let cmd = json!({
                "command": [
                    "script-message",
                    "danmaku-message",
                    &message
                ]
            });

            match pipe
                .write(serde_json::to_vec(&cmd).unwrap().as_slice())
                .and_then(|_| pipe.write(b"\n"))
            {
                Err(_) => {
                    println!("mpv IPC socket closed");
                    exit(0);
                }
                Ok(_) => (),
            }
        }
    }
}
