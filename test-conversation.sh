#!/bin/bash

# Base URL
BASE_URL="http://localhost:8080"

# Test initiating a conversation
echo "Testing conversation initiation..."
curl -X POST "$BASE_URL/conversations" \
  -H "Content-Type: application/json" \
  -d '{
    "initiator_falcon_pubkey": "alice123",
    "responder_falcon_pubkey": "bob456",
    "kyber_pubkey": "alicekyber789",
    "signature": "alicesig123"
  }'
echo -e "\n"

# Test getting pending conversations
echo "Testing pending conversations retrieval..."
curl "$BASE_URL/conversations/pending/bob456"
echo -e "\n"

# Test completing a conversation
echo "Testing conversation completion..."
curl -X POST "$BASE_URL/conversations/complete" \
  -H "Content-Type: application/json" \
  -d '{
    "initiator_falcon_pubkey": "alice123",
    "responder_falcon_pubkey": "bob456",
    "kyber_ciphertext": "encryptedstuff",
    "signature": "bobsig456"
  }'
echo -e "\n"

# Test getting pending conversations
echo "Testing pending conversations retrieval..."
curl "$BASE_URL/conversations/pending/bob456"
echo -e "\n"


