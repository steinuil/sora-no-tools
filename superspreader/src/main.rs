mod irc_source;
mod message;

use clap::{App, Arg};
use hyper::body::Bytes;
use hyper::service::{make_service_fn, service_fn};
use irc::error::Result as IrcResult;
use irc_source::{IrcSource, IrcSourceConfig};
use log::{error, info};
use message::Message;
use simple_logger::SimpleLogger;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

async fn handle_connection(
    mut receiver: broadcast::Receiver<Message>,
) -> hyper::http::Result<hyper::Response<hyper::Body>> {
    let (mut body_sender, body_channel) = hyper::Body::channel();

    tokio::spawn(async move {
        loop {
            let msg = receiver.recv().await.unwrap();
            match body_sender
                .send_data(Bytes::from(format!("{}", msg.message)))
                .await
            {
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

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    let matches = App::new("superspreader")
        .arg(
            Arg::with_name("nick")
                .long("nick")
                .takes_value(true)
                .default_value("superspreader-bot"),
        )
        .arg(
            Arg::with_name("channel")
                .long("channel")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("irc-server")
                .long("irc-server")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("irc-port")
                .long("irc-port")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("address")
                .long("address")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let sender: Arc<broadcast::Sender<Message>> = Arc::new(broadcast::channel(16).0);

    let irc_future = irc_loop(
        IrcSourceConfig {
            nickname: matches.value_of("nick").unwrap().to_owned(),
            password: None,
            channel: matches.value_of("channel").unwrap().to_owned(),
            server: matches.value_of("irc-server").unwrap().to_owned(),
            port: matches
                .value_of("irc-port")
                .map(str::parse::<u16>)
                .transpose()
                .unwrap(),
        },
        Arc::clone(&sender),
    );

    let addr = SocketAddr::from(
        format!(
            "{}:{}",
            matches.value_of("address").unwrap(),
            matches.value_of("port").unwrap()
        )
        .parse::<SocketAddr>()
        .unwrap(),
    );
    info!("starting server on {}", addr);
    let http_future = http_loop(&addr, Arc::clone(&sender));

    let (_, irc_future) = tokio::join!(http_future, irc_future);
    irc_future.unwrap();
}
