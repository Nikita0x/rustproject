use chrono::{NaiveDateTime, Utc};
use dotenv::dotenv;
use reqwest::Client;
use rodio::{source::Source, Decoder, OutputStream};
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::{env, time::Duration};
use tokio::time::sleep;
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

    let mut already_live = false; // Flag to track when the streamer goes live
    let mut count = 0;

    loop {
        let is_live = check_streamer_status(&client_id, &oauth_token, &streamer_name).await;
        count += 1;

        if is_live && !already_live {
            already_live = true; // Mark that we detected the stream is live

            if !message.is_empty() {
                // TODO: Added simple sound when stream went Live and sent message
                // Get an output stream handle to the default physical sound device.
                // Note that no sound will be played if _stream is dropped
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                // Load a sound from a file, using a path relative to Cargo.toml
                let file = BufReader::new(File::open("examples/music.ogg").unwrap());
                // Decode that sound file into a source
                let source = Decoder::new(file).unwrap();
                // Play the sound directly on the device
                stream_handle.play_raw(source.convert_samples());

                println!("Sending message...");
                send_message_to_chat(&username, &oauth_token, &streamer_name, &message).await;
                println!("✅ Message sent! Exiting...");
                break; // Exit after sending the message
            } else {
                // TODO: доделать тут чтобы был постоянный конекшн если нету сообщения и я видел чужие типо в чате.
                println!("⚠️ Stream is LIVE, but no message is set. Skipping message send.");
            }
        } else if !is_live {
            already_live = false; // Reset flag if the streamer is offline
        }

        print!("Request №: {} ", count);
        sleep(Duration::from_millis(500)).await; // Wait 500ms before next check
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
                    // ✅ Check if `data` is empty before accessing `data[0]`
                    if stream_data.data.is_empty() {
                        println!("📴 Stream is OFFLINE.");
                        return false; // Stream is offline
                    }

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
