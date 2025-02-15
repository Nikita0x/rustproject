use chrono::{NaiveDateTime, Utc};
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::{env, fmt::format};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};

#[derive(Debug, Deserialize)]
struct StreamData {
    data: Vec<StreamInfo>,
}

#[derive(Debug, Deserialize)]
struct StreamInfo {
    user_name: String,
    title: String,
    viewer_count: u32,
    started_at: String,
    language: String,
    thumbnail_url: String,
    tags: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env
    dotenv().ok();

    let client_id = env::var("TWITCH_CLIENT_ID").expect("TWITCH_CLIENT_ID not found");
    let oauth_token = env::var("TWITCH_OAUTH_TOKEN").expect("TWITCH_OAUTH_TOKEN not found");
    let streamer_name = env::var("TWITCH_STREAMER_NAME").expect("TWITCH_STREAMER_NAME not found");
    let username = env::var("TWITCH_USERNAME").expect("TWITCH_USERNAME not found");
    let message = env::var("TWITCH_MESSAGE").expect("TWITCH_MESSAGE not found");

    // Check if the streamer is live
    let is_live = check_streamer_status(&client_id, &oauth_token, &streamer_name).await;

    if is_live {
        if !message.is_empty() {
            send_message_to_chat(&username, &oauth_token, &streamer_name, &message).await;
        }
    }
}

async fn check_streamer_status(client_id: &str, oauth_token: &str, streamer_name: &str) -> bool {
    let url = format!(
        "https://api.twitch.tv/helix/streams?user_login={}",
        streamer_name
    );

    // Create a new instance for reqwest to perform HTTP calls
    let client = Client::new();
    let response = client
        .get(&url)
        .header("Client-ID", client_id)
        .header("Authorization", format!("Bearer {}", oauth_token))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response".to_string());
            println!("Status: {}", status);

            // Deserialize the body to your StreamData struct
            if status.is_success() {
                if let Ok(stream_data) = serde_json::from_str::<StreamData>(&body) {
                    let stream_info = &stream_data.data[0];

                    //Format time into more readable format
                    let started_at = &stream_info.started_at;
                    let parsed_datetime = NaiveDateTime::parse_from_str(started_at, "%+")
                        .expect("Failed to parse date time");
                    // Format the parsed datetime
                    let formatted_datetime = parsed_datetime.format("%H:%M:%S").to_string();

                    println!("🎉 Stream is LIVE!");
                    println!("Title: {}", stream_info.title);
                    println!("Viewer count: {}", stream_info.viewer_count);
                    println!("Started at: {}", formatted_datetime);
                    return !stream_data.data.is_empty();
                } else {
                    eprintln!("❌ Failed to parse stream info.");
                }
            } else {
                eprintln!("⚠️ Twitch API returned an error.");
            }
        }
        Err(e) => {
            eprintln!("Request failed: {}", e);
        }
    }

    return false;
}

async fn send_message_to_chat(username: &str, oauth_token: &str, channel: &str, message: &str) {
    // Configure the Twitch IRC client
    let config = ClientConfig::new_simple(StaticLoginCredentials::new(
        username.to_string(),
        Some(oauth_token.to_string()),
    ));
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    // Spawn a task to handle incoming messages (optional)
    tokio::spawn(async move {
        while let Some(message) = incoming_messages.recv().await {
            println!("Received message: {:?}", message);
        }
    });

    // Join the streamer's channel
    client.join(channel.to_string());

    // Send the message to the chat
    client
        .say(channel.to_string(), message.to_string())
        .await
        .expect("Failed to send message");

    println!("Message sent: {}", message);

    // Keep the client running to receive messages (optional)
    // You can adjust the sleep duration as needed
    tokio::time::sleep(tokio::time::Duration::from_secs(5000)).await;
}
