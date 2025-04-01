#![feature(slice_as_chunks)]
#![feature(new_zeroed_alloc)]

mod command;
mod directory_entry;
mod node;
mod node_kind;
mod nodefs;
mod util;

use clap::Parser;
use command::{Command, Operation};
use nodefs::NodeFS;
use serenity::prelude::*;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Expected .env file with BOT_TOKEN and DATA_CHANNEL_ID");

    let command = Command::parse();

    let token = std::env::var("BOT_TOKEN")
        .expect("Requires Discord bot token in environment variable 'BOT_TOKEN'");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let channel: u64 = std::env::var("DATA_CHANNEL_ID")
        .expect("Requires data channel ID in environment variable 'DATA_CHANNEL_ID'")
        .parse()
        .expect("Expected a valid u64 discord channel ID");

    let client = Client::builder(token, intents)
        .await
        .expect("Failed to create client");

    let mut nodefs = NodeFS::new(channel, client);
    nodefs.setup().await;

    match command.operation {
        Operation::Ls { path } => nodefs.ls(path).await,
        Operation::Upload {
            source,
            destination,
        } => nodefs.upload(source, destination).await,
        Operation::Download {
            source,
            destination,
        } => nodefs.download(source, destination).await,
        Operation::Rm {
            path,
            quick,
            recursive,
        } => nodefs.rm(path, quick, recursive).await,
        Operation::Mv {
            source,
            destination,
        } => nodefs.mv(source, destination).await,
        Operation::Replace {
            quick,
            source,
            destination,
        } => nodefs.replace(source, destination, quick).await,
        Operation::Rename { old, new } => nodefs.rename(old, new).await,
        Operation::Mkdir { path } => nodefs.mkdir(path).await,
    };
}
