mod irc_source;
mod message;

use hyper::{
    body::Bytes,
    service::{make_service_fn, service_fn},
};
use irc::error::Result as IrcResult;
use irc_source::{IrcSource, IrcSourceConfig};
use log::{error, info};
use message::Message;
use simple_logger::SimpleLogger;
use std::{
    convert::Infallible,
    net::{Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};
use structopt::StructOpt;
use tokio::sync::broadcast;

async fn handle_connection(
    mut receiver: broadcast::Receiver<Message>,
) -> hyper::http::Result<hyper::Response<hyper::Body>> {
    let (mut body_sender, body_channel) = hyper::Body::channel();

    tokio::spawn(async move {
        loop {
            let msg = receiver.recv().await.unwrap();
            match body_sender.send_data(Bytes::from(msg.message)).await {
                Ok(()) => (),
                Err(err) if err.is_closed() => break,
                _ => break,
            }
        }
    });

    hyper::Response::builder()
        .header("Content-Type", "text/plain")
        .header("Access-Control-Allow-Origin", "*")
        .body(body_channel)
}

async fn handle(
    req: hyper::Request<hyper::Body>,
    receiver: tokio::sync::broadcast::Receiver<Message>,
) -> hyper::http::Result<hyper::Response<hyper::Body>> {
    match req.method() {
        &hyper::Method::GET => handle_connection(receiver).await,
        &hyper::Method::OPTIONS => hyper::Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .body(hyper::Body::empty()),
        _ => hyper::Response::builder()
            .status(hyper::StatusCode::METHOD_NOT_ALLOWED)
            .body(hyper::Body::empty()),
    }
}

async fn http_loop(addr: &SocketAddr, sender: Arc<broadcast::Sender<Message>>) {
    let make_service = make_service_fn(|_| {
        let sender = Arc::clone(&sender);
        async { Ok::<_, Infallible>(service_fn(move |req| handle(req, sender.subscribe()))) }
    });

    let server = hyper::Server::bind(addr).serve(make_service);

    if let Err(err) = server.await {
        error!("server error: {}", err)
    }
}

async fn irc_loop(
    config: IrcSourceConfig,
    sender: Arc<broadcast::Sender<Message>>,
) -> IrcResult<()> {
    IrcSource::connect(config)
        .await?
        .on_channel_message(|message| {
            let _ = sender.send(message);
        })
        .await?;

    Ok(())
}

/// Superspreader connects to a channel on an IRC server
/// and then starts an HTTP server that will stream the messages sent to the channel
/// to all clients connected to it via chunked transfer encoding.
#[derive(StructOpt)]
struct Options {
    #[structopt(long, default_value = "superspreader-bot")]
    pub nick: String,

    #[structopt(long)]
    pub channel: String,

    #[structopt(long)]
    pub irc_server: String,

    #[structopt(long)]
    pub irc_port: Option<u16>,

    #[structopt(long)]
    pub address: String,

    #[structopt(long)]
    pub port: u16,
}

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    let opts = Options::from_args();

    let bind_address =
        SocketAddr::new(Ipv4Addr::from_str(&opts.address).unwrap().into(), opts.port);

    let sender: Arc<broadcast::Sender<Message>> = Arc::new(broadcast::channel(16).0);

    info!("connecting to {} on {}", opts.channel, opts.irc_server);
    let irc_future = irc_loop(
        IrcSourceConfig {
            nickname: opts.nick,
            password: None,
            channel: opts.channel,
            server: opts.irc_server,
            port: opts.irc_port,
        },
        Arc::clone(&sender),
    );

    info!("starting server on {}", bind_address);
    let http_future = http_loop(&bind_address, Arc::clone(&sender));

    let (_, irc_future) = tokio::join!(http_future, irc_future);
    irc_future.unwrap();
}
