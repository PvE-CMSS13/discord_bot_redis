use dotenvy::dotenv;

use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter, CreateMessage, Timestamp};
use tokio::try_join;

use futures_util::StreamExt;

use serde::{Deserialize, Serialize};

use std::{env, sync::Arc};

const ENV_LOOKUP_PAIRS: [(&str, &str, fn(payload: String) -> Option<CreateMessage>); 4] = [
    (
        "REDIS_ASAY_SUBSCRIPTION",
        "REDIS_ASAY_SUBSCRIPTION_DISCORD_CHANNEL_OUTPUT",
        handle_asay_subscription,
    ),
    (
        "REDIS_ACCESS_SUBSCRIPTION",
        "REDIS_ACCESS_SUBSCRIPTION_DISCORD_CHANNEL_OUTPUT",
        handle_access_subscription,
    ),
    (
        "REDIS_ROUND_SUBSCRIPTION",
        "REDIS_ROUND_SUBSCRIPTION_DISCORD_CHANNEL_OUTPUT",
        handle_round_subscription,
    ),
    (
        "REDIS_META_SUBSCRIPTION",
        "REDIS_META_SUBSCRIPTION_DISCORD_CHANNEL_OUTPUT",
        handle_meta_subscription,
    ),
];

#[tokio::main]
async fn main() {
    match dotenv() {
        Ok(_) => {}
        Err(_error) => {
            println!("No local .env found. This is normal if running in docker. Continuing.");
        }
    }

    let discord_token = match env::var("DISCORD_TOKEN") {
        Ok(token) => token,
        Err(error) => {
            println!("Unable to get discord token for redis functionality. Disabling redis functionality. Error: {error:?}");
            return;
        }
    };

    let discord_connection = Arc::new(serenity::all::Http::new(discord_token.as_str()));
    println!("Redis bot connected to discord.");

    let redis_url = match env::var("REDIS_URL") {
        Ok(unwrapped_url) => unwrapped_url,
        Err(error) => {
            println!("No redis URL found. Disabling redis functionality. Error: {error:?}");
            return;
        }
    };

    let redis_client = match redis::Client::open(redis_url) {
        Ok(client) => client,
        Err(error) => {
            println!("Unable to get redis client. Disabling redis functionality. Error: {error:?}");
            return;
        }
    };

    let mut tasks = Vec::new();

    for (subscription_env_var, discord_output_env_var, handler) in ENV_LOOKUP_PAIRS {
        tasks.push(tokio::spawn(setup_subscriptions(
            redis_client.clone(),
            subscription_env_var,
            discord_output_env_var,
            handler,
            discord_connection.clone(),
        )));
    }

    //TODO: Auto recovery on error
    for task in tasks {
        match try_join!(task) {
            Ok(_) => {}
            Err(error) => {
                println!("Redis pubsub errored. Closing redis. Error: {error:?}");
                return;
            }
        }
    }
}

async fn setup_subscriptions(
    redis_client: redis::Client,
    subscription_env_var: &str,
    discord_output_env_var: &str,
    handler: fn(payload: String) -> Option<CreateMessage>,
    discord_connection: Arc<serenity::all::Http>,
) {
    let subscription_env_value = match std::env::var(subscription_env_var) {
        Ok(result) => result,
        Err(error) => {
            println!(
                "Unable to find {subscription_env_var}. Disabling subscription. Error {error:?}"
            );
            return;
        }
    };

    let discord_output_channel_value = match std::env::var(discord_output_env_var) {
        Ok(result) => result,
        Err(error) => {
            println!(
                "Unable to find {discord_output_env_var}. Disabling subscription. Error {error:?}"
            );
            return;
        }
    };

    let discord_output_channel_value_parsed: u64 = match discord_output_channel_value.parse() {
        Ok(result) => result,
        Err(error) => {
            println!("Unable to cast environment variable {discord_output_env_var} to u64. Disabling subscription. Error: {error:?}");
            return;
        }
    };

    let discord_output_channel = serenity::all::ChannelId::new(discord_output_channel_value_parsed);

    let mut redis_pubsub = match redis_client.get_async_pubsub().await {
        Ok(connection) => connection,
        Err(error) => {
            println!("Unable to get async pubsub connection for {subscription_env_value}. Disabling. Error: {error:?}");
            return;
        }
    };

    match redis_pubsub.subscribe(subscription_env_value.clone()).await {
        Ok(_) => {
            println!("Successful subscription to {subscription_env_var}. Listening for data.");
        }
        Err(error) => {
            println!(
                "Unable to subscribe to {subscription_env_value}. Disabling. Error: {error:?}"
            );
            return;
        }
    }

    let mut redis_pubsub_stream = redis_pubsub.on_message();

    loop {
        let redis_message = match redis_pubsub_stream.next().await {
            Some(message) => message,
            None => continue,
        };

        let redis_payload: String = match redis_message.get_payload() {
            Ok(payload) => payload,
            Err(error) => {
                println!("Unable to get payload from redis message. Error: {error:?} \nMessage: {redis_message:?}");
                continue;
            }
        };

        println!("Found redis payload. Output: {redis_payload:?}");

        match handler(redis_payload) {
            Some(message) => {
                if let Err(error) = discord_output_channel
                    .send_message(discord_connection.clone(), message)
                    .await
                {
                    println!("Error sending message: {error:?}");
                }
            }
            None => {}
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct AsaySubscription {
    source: String,
    #[serde(default)]
    round_id: String,
    author: String,
    message: String,
    #[serde(default)]
    admin: u8,
    rank: String,
}

fn handle_asay_subscription(payload: String) -> Option<CreateMessage> {
    let deserialized_payload: AsaySubscription = match serde_json::from_str(&payload) {
        Ok(deserialized) => deserialized,
        Err(error) => {
            println!("Unable to deserialize from String to AsaySubscription. Error: {error:?}");
            return None;
        }
    };

    //TODO: Convert this to const when we publish on this channel later
    if deserialized_payload.source == "discord" {
        return None;
    }

    let message = CreateMessage::new().embed(
        CreateEmbed::new()
            .title(deserialized_payload.author)
            .description(deserialized_payload.message)
            .footer(CreateEmbedFooter::new(format!(
                "{}@{}",
                deserialized_payload.rank, deserialized_payload.source
            )))
            .timestamp(Timestamp::now())
            .color(Colour::from_rgb(124, 68, 12)),
    );

    return Some(message);
}
//TODO: Implement access
fn handle_access_subscription(payload: String) -> Option<CreateMessage> {
    None
}
//TODO: Implement access
fn handle_round_subscription(payload: String) -> Option<CreateMessage> {
    None
}
//TODO: Implement access
fn handle_meta_subscription(payload: String) -> Option<CreateMessage> {
    None
}

//TODO: Redis publish discord -> server messages
