use chrono::{NaiveDateTime, Utc};
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::{env, fmt::format};

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

    let url = format!(
        "https://api.twitch.tv/helix/streams?user_login={}",
        streamer_name
    );

    // create new instance for reqwest to do http calls
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Client-ID", &client_id)
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
            println!("Status: {status}");

            // Deserialize the body to your StreamData struct
            if status.is_success() {
                if let Ok(stream_data) = serde_json::from_str::<StreamData>(&body) {
                    if !stream_data.data.is_empty() {
                        let stream_info = &stream_data.data[0]; // Assuming the first stream in the list

                        // Format time into more readable format
                        let started_at = &stream_info.started_at;
                        let parsed_datetime = NaiveDateTime::parse_from_str(started_at, "%+")
                            .expect("Failed to parse date time");
                        // Format the parsed datetime
                        let formatted_datetime =
                            parsed_datetime.format("%A, %B %d, %Y %H:%M:%S").to_string();

                        println!("🎉 Stream is LIVE!");
                        println!("Title: {}", stream_info.title);
                        println!("Viewer count: {}", stream_info.viewer_count);
                        println!("Started at: {}", formatted_datetime);
                        println!("Language: {}", stream_info.language);
                        println!("Tags: {:?}", stream_info.tags);
                        println!("Thumbnail URL: {}", stream_info.thumbnail_url);
                    } else {
                        println!("📴 Stream is OFFLINE.");
                    }
                } else {
                    eprintln!("❌ Failed to parse stream info.");
                }
            } else {
                eprintln!("⚠️ Twitch API returned an error.");
            }
        }
        Err(e) => {
            eprintln!("Request failed: {e}");
        }
    }

    println!("Client ID: {}", client_id);
    println!("OAuth Token: {}", oauth_token);
    println!("Streamer: {}", streamer_name);
}
