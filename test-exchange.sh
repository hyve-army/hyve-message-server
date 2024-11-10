#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Base URL for the API
BASE_URL="https://hyveapi.net"
#BASE_URL="http://localhost:8080"

# Test data
INITIATOR_FALCON_PUBKEY="initiatorFalconKey123"
RESPONDER_FALCON_PUBKEY="responderFalconKey456"
INITIATOR_KYBER_PUBKEY="initiatorKyberKey789"
INITIATOR_SIGNATURE="initiatorSignature123"
RESPONDER_SIGNATURE="responderSignature456"
ENCAPSULATED_SECRET="encapsulatedSecret789"

echo -e "${GREEN}Starting key exchange protocol test...${NC}"

# Step 1: Initialize key exchange
echo -e "\n${GREEN}1. Initializing key exchange...${NC}"
INIT_RESPONSE=$(curl -s -X POST "$BASE_URL/exchanges/init" \
  -H "Content-Type: application/json" \
  -d "{
    \"initiator_falcon_pubkey\": \"$INITIATOR_FALCON_PUBKEY\",
    \"responder_falcon_pubkey\": \"$RESPONDER_FALCON_PUBKEY\",
    \"initiator_kyber_pubkey\": \"$INITIATOR_KYBER_PUBKEY\",
    \"initiator_signature\": \"$INITIATOR_SIGNATURE\"
  }")

echo "Response: $INIT_RESPONSE"

# Step 2: Check for initiated exchanges (responder's perspective)
echo -e "\n${GREEN}2. Checking initiated exchanges for responder...${NC}"
INITIATED_RESPONSE=$(curl -s -X GET "$BASE_URL/exchanges/initiated/$RESPONDER_FALCON_PUBKEY")
echo "Response: $INITIATED_RESPONSE"

# Step 3: Pair the exchange (responder's action)
echo -e "\n${GREEN}3. Pairing the exchange...${NC}"
PAIR_RESPONSE=$(curl -s -X POST "$BASE_URL/exchanges/pair" \
  -H "Content-Type: application/json" \
  -d "{
    \"initiator_falcon_pubkey\": \"$INITIATOR_FALCON_PUBKEY\",
    \"responder_falcon_pubkey\": \"$RESPONDER_FALCON_PUBKEY\",
    \"encapsulated_secret\": \"$ENCAPSULATED_SECRET\",
    \"responder_signature\": \"$RESPONDER_SIGNATURE\"
  }")

echo "Response: $PAIR_RESPONSE"

# Step 4: Check paired exchanges (initiator's perspective)
echo -e "\n${GREEN}4. Checking paired exchanges for initiator...${NC}"
PAIRED_RESPONSE=$(curl -s -X GET "$BASE_URL/exchanges/paired/$INITIATOR_SIGNATURE")
echo "Response: $PAIRED_RESPONSE"

# Step 5: Complete the exchange
echo -e "\n${GREEN}5. Completing the exchange...${NC}"
COMPLETE_RESPONSE=$(curl -s -X POST "$BASE_URL/exchanges/complete" \
  -H "Content-Type: application/json" \
  -d "{
    \"initiator_falcon_pubkey\": \"$INITIATOR_FALCON_PUBKEY\",
    \"responder_falcon_pubkey\": \"$RESPONDER_FALCON_PUBKEY\"
  }")

echo "Response: $COMPLETE_RESPONSE"

# Step 6: Check completed exchanges
echo -e "\n${GREEN}6. Checking completed exchanges for responder...${NC}"
COMPLETED_RESPONSE=$(curl -s -X GET "$BASE_URL/exchanges/complete/$RESPONDER_FALCON_PUBKEY")
echo "Response: $COMPLETED_RESPONSE"

echo -e "\n${GREEN}Key exchange protocol test completed${NC}"
