use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use chrono::{DateTime, Utc};
use log::info;


#[derive(Clone, Serialize, Deserialize)]
struct Conversation {
    initiator_falcon_pubkey: String,
    responder_falcon_pubkey: String,
    initiator_kyber_pubkey: String,
    encapsulated_secret: Option<String>,
    status: ConversationStatus,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
enum ConversationStatus {
    Pending,    // Waiting for responder
    Complete,   // Both parties have exchanged keys
    Expired     // Timed out waiting for response
}

#[derive(Deserialize)]
struct InitConversationRequest {
    initiator_falcon_pubkey: String,
    responder_falcon_pubkey: String,
    kyber_pubkey: String,
    signature: String,
}

#[derive(Deserialize)]
struct CompleteConversationRequest {
    initiator_falcon_pubkey: String,
    responder_falcon_pubkey: String,
    kyber_ciphertext: String,
    signature: String,
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
    conversations: Mutex<HashMap<String, Conversation>>,
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

// Initialize a conversation
async fn init_conversation(
    data: web::Data<AppState>,
    req: web::Json<InitConversationRequest>,
) -> impl Responder {
    info!("Received init conversation request from: {}", req.initiator_falcon_pubkey);
    let mut conversations = data.conversations.lock().unwrap();

    let conversation_id = format!("{}:{}", req.initiator_falcon_pubkey, req.responder_falcon_pubkey);

    // Check if there's already an active conversation
    // TODO: enable ratchet here under some conditions
    if let Some(existing) = conversations.get(&conversation_id) {
        if existing.status == ConversationStatus::Complete {
            return HttpResponse::Conflict().json("Active conversation already exists");
        }
    }

    let conversation = Conversation {
        initiator_falcon_pubkey: req.initiator_falcon_pubkey.clone(),
        responder_falcon_pubkey: req.responder_falcon_pubkey.clone(),
        initiator_kyber_pubkey: req.kyber_pubkey.clone(),
        encapsulated_secret: None,
        status: ConversationStatus::Pending,
        created_at: Utc::now(),
        completed_at: None,
    };

    conversations.insert(conversation_id.clone(), conversation.clone());

    HttpResponse::Ok().json(conversation)
}

// Complete the conversation with encapsulated key
async fn complete_conversation(
    data: web::Data<AppState>,
    req: web::Json<CompleteConversationRequest>,
) -> impl Responder {
    info!("Received complete conversation request from: {}", req.responder_falcon_pubkey);
    let mut conversations = data.conversations.lock().unwrap();

    // Form the lookup key from the initiator and responder public keys
    let conversation_id = format!("{}:{}", req.initiator_falcon_pubkey, req.responder_falcon_pubkey);

    if let Some(mut conversation) = conversations.get_mut(&conversation_id) {
        if conversation.status != ConversationStatus::Pending {
            return HttpResponse::BadRequest().json("Conversation not in pending state");
        }

        conversation.encapsulated_secret = Some(req.kyber_ciphertext.clone());
        conversation.status = ConversationStatus::Complete;
        conversation.completed_at = Some(Utc::now());

        HttpResponse::Ok().json(conversation)
    } else {
        HttpResponse::NotFound().finish()
    }
}


// Get pending conversations where I'm the responder
async fn get_pending_conversations(
    data: web::Data<AppState>,
    responder_falcon_pubkey: web::Path<String>,
) -> impl Responder {
    let conversations = data.conversations.lock().unwrap();

    let pending: Vec<_> = conversations
        .iter()
        .filter(|(_, conv)| {
            conv.responder_falcon_pubkey == *responder_falcon_pubkey
                && conv.status == ConversationStatus::Pending
        })
        .map(|(id, conv)| (id.clone(), conv.clone()))
        .collect();

    HttpResponse::Ok().json(pending)
}

// Cleanup expired conversations periodically
async fn cleanup_expired_conversations(data: web::Data<AppState>) {
    let mut conversations = data.conversations.lock().unwrap();
    let expiry_time = Utc::now() - chrono::Duration::hours(24);

    conversations.retain(|_, conv| {
        if conv.status == ConversationStatus::Pending && conv.created_at < expiry_time {
            conv.status = ConversationStatus::Expired;
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
        conversations: Mutex::new(HashMap::new()),
        messages: Mutex::new(HashMap::new()),
    });

    println!("Starting server on http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/conversations", web::post().to(init_conversation))
            .route("/conversations/complete", web::post().to(complete_conversation))
            .route("/conversations/pending/{pubkey}", web::get().to(get_pending_conversations))
            .route("/messages", web::post().to(store_message))
            .route("/messages/{recipient}", web::get().to(get_messages))
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}