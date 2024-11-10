use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use chrono::{DateTime, Utc};
use log::info;

// TODO: none of these methods have authentication built in

#[derive(Clone, Serialize, Deserialize)]
struct KeyExchange {
    initiator_falcon_pubkey: String,
    responder_falcon_pubkey: String,
    initiator_kyber_pubkey: String,
    initiator_signature: String,
    responder_signature: Option<String>,
    encapsulated_secret: Option<String>,
    status: KeyExchangeStatus,
    created_at: DateTime<Utc>,
    paired_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
enum KeyExchangeStatus {
    Initiated,    // Waiting for responder
    Paired,      // Responder pairs their key, sends encapsulated secret
    Complete,   // Both parties have exchanged keys
    Expired     // Timed out waiting for response
}

#[derive(Deserialize)]
struct InitKeyExchangeRequest {
    initiator_falcon_pubkey: String,
    responder_falcon_pubkey: String,
    initiator_kyber_pubkey: String,
    initiator_signature: String,
}

#[derive(Deserialize)]
struct PairKeyExchangeRequest {
    initiator_falcon_pubkey: String,
    responder_falcon_pubkey: String,
    encapsulated_secret: String,
    responder_signature: String,
}

#[derive(Deserialize)]
struct CompleteKeyExchangeRequest {
    initiator_falcon_pubkey: String,
    responder_falcon_pubkey: String,
}

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
    key_exchanges: Mutex<HashMap<String, KeyExchange>>,
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

// Initialize a key exchange
async fn init_key_exchange(
    data: web::Data<AppState>,
    req: web::Json<InitKeyExchangeRequest>,
) -> impl Responder {
    info!("Received init key exchange request from: {} to: {}", req.initiator_falcon_pubkey, req.responder_falcon_pubkey);
    let mut key_exchanges = data.key_exchanges.lock().unwrap();

    let exchange_id = format!("{}:{}", req.initiator_falcon_pubkey, req.responder_falcon_pubkey);

    // Check if there's already an active conversation
    // TODO: enable ratchet here under some conditions
    if let Some(existing) = key_exchanges.get(&exchange_id) {
        if existing.status == KeyExchangeStatus::Complete {
            return HttpResponse::Conflict().json("Active key exchange already exists");
        }
    }

    let key_exchange = KeyExchange {
        initiator_falcon_pubkey: req.initiator_falcon_pubkey.clone(),
        responder_falcon_pubkey: req.responder_falcon_pubkey.clone(),
        initiator_kyber_pubkey: req.initiator_kyber_pubkey.clone(),
        initiator_signature: req.initiator_signature.clone(),
        responder_signature: None,
        encapsulated_secret: None,
        status: KeyExchangeStatus::Initiated,
        created_at: Utc::now(),
        paired_at: None,
        completed_at: None,
    };

    key_exchanges.insert(exchange_id.clone(), key_exchange.clone());

    HttpResponse::Ok().json(key_exchange)
}

async fn pair_exchange(
    data: web::Data<AppState>,
    req: web::Json<PairKeyExchangeRequest>,
) -> impl Responder {
    info!("Received pair key exchange request from: {} to: {}", req.responder_falcon_pubkey, req.initiator_falcon_pubkey);
    let mut key_exchanges = data.key_exchanges.lock().unwrap();

    // Form the lookup key from the initiator and responder public keys
    let exchange_id = format!("{}:{}", req.initiator_falcon_pubkey, req.responder_falcon_pubkey);

    if let Some(mut key_exchange) = key_exchanges.get_mut(&exchange_id) {
        if key_exchange.status != KeyExchangeStatus::Initiated {
            return HttpResponse::BadRequest().json("Key exchange not in Initiated state");
        }

        key_exchange.responder_signature = Some(req.responder_signature.clone());
        key_exchange.encapsulated_secret = Some(req.encapsulated_secret.clone());
        key_exchange.status = KeyExchangeStatus::Paired;
        key_exchange.paired_at = Some(Utc::now());

        HttpResponse::Ok().json(key_exchange)
    } else {
        HttpResponse::NotFound().finish()
    }
}

// Complete the conversation with encapsulated key
async fn complete_exchange(
    data: web::Data<AppState>,
    req: web::Json<CompleteKeyExchangeRequest>,
) -> impl Responder {
    info!("Received complete key exchange request from: {}", req.initiator_falcon_pubkey);
    let mut key_exchanges = data.key_exchanges.lock().unwrap();

    // Form the lookup key from the initiator and responder public keys
    let conversation_id = format!("{}:{}", req.initiator_falcon_pubkey, req.responder_falcon_pubkey);

    if let Some(mut key_exchange) = key_exchanges.get_mut(&conversation_id) {
        if key_exchange.status != KeyExchangeStatus::Paired {
            return HttpResponse::BadRequest().json("Key exchange not in Paired state");
        }

        key_exchange.status = KeyExchangeStatus::Complete;
        key_exchange.completed_at = Some(Utc::now());

        HttpResponse::Ok().json(key_exchange)
    } else {
        HttpResponse::NotFound().finish()
    }
}


// Get pending key_exchanges where I'm the responder
async fn get_initiated_exchanges(
    data: web::Data<AppState>,
    responder_falcon_pubkey: web::Path<String>,
) -> impl Responder {
    let key_exchanges = data.key_exchanges.lock().unwrap();

    let initiated: Vec<_> = key_exchanges
        .iter()
        .filter(|(_, conv)| {
            conv.responder_falcon_pubkey == *responder_falcon_pubkey
                && conv.status == KeyExchangeStatus::Initiated
        })
        .map(|(id, conv)| (id.clone(), conv.clone()))
        .collect();

    HttpResponse::Ok().json(initiated)
}

// Get paired key_exchanges where I'm the responder
async fn get_paired_exchanges(
    data: web::Data<AppState>,
    initiator_signature: web::Path<String>,
) -> impl Responder {
    let key_exchanges = data.key_exchanges.lock().unwrap();

    let paired: Vec<_> = key_exchanges
        .iter()
        .filter(|(_, conv)| {
            conv.initiator_signature == *initiator_signature
                && conv.status == KeyExchangeStatus::Paired
        })
        .map(|(id, conv)| (id.clone(), conv.clone()))
        .collect();

    HttpResponse::Ok().json(paired)
}


// Get completed key_exchanges where I'm the responder
async fn get_completed_exchanges(
    data: web::Data<AppState>,
    responder_falcon_pubkey: web::Path<String>,
) -> impl Responder {
    let key_exchanges = data.key_exchanges.lock().unwrap();

    let completed: Vec<_> = key_exchanges
        .iter()
        .filter(|(_, conv)| {
            conv.responder_falcon_pubkey == *responder_falcon_pubkey
                && conv.status == KeyExchangeStatus::Complete
        })
        .map(|(id, conv)| (id.clone(), conv.clone()))
        .collect();

    HttpResponse::Ok().json(completed)
}


// Cleanup expired key_exchanges periodically
async fn cleanup_expired_exchanges(data: web::Data<AppState>) {
    let mut key_exchanges = data.key_exchanges.lock().unwrap();
    let expiry_time = Utc::now() - chrono::Duration::hours(24);

    key_exchanges.retain(|_, conv| {
        if conv.status == KeyExchangeStatus::Initiated && conv.created_at < expiry_time {
            conv.status = KeyExchangeStatus::Expired;
            false
        } else {
            true
        }
    });
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));
    let app_state = web::Data::new(AppState {
        key_exchanges: Mutex::new(HashMap::new()),
        messages: Mutex::new(HashMap::new()),
    });

    println!("Starting server on http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/exchanges/init", web::post().to(init_key_exchange))
            .route("/exchanges/pair", web::post().to(pair_exchange))
            .route("/exchanges/complete", web::post().to(complete_exchange))
            .route("/exchanges/initiated/{pubkey}", web::get().to(get_initiated_exchanges))
            .route("/exchanges/paired/{pubkey}", web::get().to(get_paired_exchanges))
            .route("/exchanges/complete/{pubkey}", web::get().to(get_completed_exchanges))
            .route("/messages", web::post().to(store_message))
            .route("/messages/{recipient}", web::get().to(get_messages))
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}