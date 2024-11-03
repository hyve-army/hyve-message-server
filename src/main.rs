use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use chrono::{DateTime, Utc};

// Structure for messages
#[derive(Serialize, Deserialize, Clone)]
struct Message {
    from_pubkey: String,
    to_pubkey: String,
    ciphertext: String,
    timestamp: DateTime<Utc>,
}

// Structure to hold messages for each recipient
struct AppState {
    messages: Mutex<HashMap<String, Vec<Message>>>,
}

// Request payload for storing messages
#[derive(Deserialize)]
struct StoreMessageRequest {
    from_pubkey: String,
    to_pubkey: String,
    ciphertext: String,
}

// Store a new message
async fn store_message(
    data: web::Data<AppState>,
    message: web::Json<StoreMessageRequest>,
) -> impl Responder {
    let mut messages = data.messages.lock().unwrap();

    let new_message = Message {
        from_pubkey: message.from_pubkey.clone(),
        to_pubkey: message.to_pubkey.clone(),
        ciphertext: message.ciphertext.clone(),
        timestamp: Utc::now(),
    };

    messages
        .entry(message.to_pubkey.clone())
        .or_insert_with(Vec::new)
        .push(new_message);

    HttpResponse::Ok().json("Message stored successfully")
}

// Retrieve messages for a recipient
async fn get_messages(
    data: web::Data<AppState>,
    recipient: web::Path<String>,
) -> impl Responder {
    let messages = data.messages.lock().unwrap();

    if let Some(recipient_messages) = messages.get(&recipient.into_inner()) {
        HttpResponse::Ok().json(recipient_messages)
    } else {
        HttpResponse::Ok().json(vec![] as Vec<Message>)
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        messages: Mutex::new(HashMap::new()),
    });

    println!("Starting server on http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/messages", web::post().to(store_message))
            .route("/messages/{recipient}", web::get().to(get_messages))
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}