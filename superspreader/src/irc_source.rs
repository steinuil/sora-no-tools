use crate::message::Message;
use futures::StreamExt;
use irc::{
    client::{
        prelude::{Command, Config, Prefix},
        Client,
    },
    error::Result as IrcResult,
};
use log::info;

#[derive(Clone)]
pub struct IrcSourceConfig {
    pub nickname: String,
    pub password: Option<String>,
    pub server: String,
    pub port: Option<u16>,
    pub channel: String,
}

pub struct IrcSource {
    pub client: Client,
    pub config: IrcSourceConfig,
}

impl IrcSource {
    pub async fn connect(config: IrcSourceConfig) -> IrcResult<IrcSource> {
        let client = Client::from_config(Config {
            nickname: Some(config.nickname.to_owned()),
            password: config.password.clone(),
            channels: vec![config.channel.to_owned()],
            server: Some(config.server.to_owned()),
            port: config.port,
            ..Config::default()
        })
        .await?;
        client.identify()?;

        Ok(IrcSource { client, config })
    }

    pub async fn on_channel_message<F: Fn(Message) -> ()>(&mut self, handler: F) -> IrcResult<()> {
        let mut stream = self.client.stream()?;

        while let Some(ref message) = stream.next().await.transpose()? {
            match (&message.prefix, &message.command) {
                (Some(Prefix::Nickname(nick, _, _)), Command::PRIVMSG(ref target, ref text))
                    if target == &self.config.channel =>
                {
                    info!("<{}> {}", nick, text);
                    handler(Message {
                        nick: nick.to_owned(),
                        message: text.to_owned(),
                    })
                }
                _ => (),
            }
        }

        Ok(())
    }
}
