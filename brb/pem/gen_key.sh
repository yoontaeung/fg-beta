#!/bin/bash

# Number of key pairs to generate
num_key_pairs=100

for ((i=0; i<num_key_pairs; i++)); do
    # Generate an Ed25519 private key and save it to test-priv-{index}.pem
    index_padded=$(printf "%02d" $i)
    openssl genpkey -algorithm ed25519 -out "priv-$index_padded.pem"

    # Extract the public key from the private key and save it to test-pub-{index}.pem
    openssl pkey -in "priv-$index_padded.pem" -pubout -out "pub-$index_padded.pem"
done
