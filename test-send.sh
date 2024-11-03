curl -X POST https://hyveapi.net/messages \
  -H "Content-Type: application/json" \
  -d '{"from_pubkey":"sender123","to_pubkey":"recipient456","ciphertext":"encrypted_data_here"}'
