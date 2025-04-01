use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use serenity::{
    Client,
    all::{ChannelId, CreateMessage, EditChannel, EditMessage, GuildChannel, MessageId},
};

pub fn progress_bar(limit: u64) -> ProgressBar {
    let bar = ProgressBar::new(limit).with_style(
        ProgressStyle::with_template(
            "  [{elapsed}] {wide_bar} [{binary_bytes}/{binary_total_bytes} - {percent}%]  ",
        )
        .unwrap()
        .progress_chars("##-"),
    );
    bar.enable_steady_tick(Duration::from_millis(100));

    bar
}

pub fn spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner()
        .with_style(ProgressStyle::with_template("  {msg} {spinner}  ").unwrap());
    spinner.enable_steady_tick(Duration::from_millis(250));

    spinner
}

pub fn file_delete_progress(limit: u64) -> ProgressBar {
    let spinner = ProgressBar::new(limit).with_style(
        ProgressStyle::with_template("  [Blocks {pos}/{len}] Deleting {msg}  ").unwrap(),
    );

    spinner.enable_steady_tick(Duration::from_millis(250));

    spinner
}

pub async fn get_guild_channel(
    client: &Client,
    channel_id: ChannelId,
) -> serenity::Result<GuildChannel> {
    channel_id
        .to_channel(&client.http)
        .await?
        .guild()
        .ok_or(serenity::Error::Other("Failed to get guild channel"))
}

pub async fn send_message(
    client: &Client,
    channel_id: ChannelId,
    message: CreateMessage,
) -> serenity::Result<MessageId> {
    Ok(channel_id.send_message(&client.http, message).await?.id)
}

pub async fn edit_message(
    client: &Client,
    channel_id: ChannelId,
    message_id: MessageId,
    message: EditMessage,
) -> serenity::Result<()> {
    channel_id
        .edit_message(&client.http, message_id, message)
        .await?;

    Ok(())
}

pub async fn delete_message(
    client: &Client,
    channel_id: ChannelId,
    message_id: MessageId,
) -> serenity::Result<()> {
    channel_id.delete_message(&client.http, message_id).await
}

pub async fn edit_channel_topic(
    client: &Client,
    channel_id: ChannelId,
    topic: String,
) -> serenity::Result<GuildChannel> {
    channel_id
        .edit(&client.http, EditChannel::new().topic(topic))
        .await
}

pub async fn read_attachment(
    client: &Client,
    channel_id: ChannelId,
    message_id: MessageId,
) -> serenity::Result<Vec<u8>> {
    client
        .http
        .get_message(channel_id, message_id)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Failed to get message '{}' from channel '{}': {e}",
                message_id.get(),
                channel_id.get()
            )
        })
        .attachments
        .first()
        .unwrap_or_else(|| {
            panic!(
                "Message '{}' from channel '{}' should contain an attachment of block data",
                message_id.get(),
                channel_id.get()
            )
        })
        .download()
        .await
}
