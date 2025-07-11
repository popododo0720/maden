#!/bin/bash

# This script generates a self-signed SSL certificate for local development.

# Set variables
SSL_DIR="ssl"
KEY_FILE="${SSL_DIR}/key.pem"
CERT_FILE="${SSL_DIR}/cert.pem"
DAYS_VALID=365

# Create the ssl directory if it doesn't exist
mkdir -p ${SSL_DIR}

# Check if openssl is installed
if ! [ -x "$(command -v openssl)" ]; then
  echo 'Error: openssl is not installed.' >&2
  exit 1
fi

# Generate the private key and certificate
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "${KEY_FILE}" \
  -out "${CERT_FILE}" \
  -days "${DAYS_VALID}" \
  -subj "/C=US/ST=California/L=San Francisco/O=MadenDev/CN=localhost"

echo "SSL certificate and key generated successfully."
echo "Key file: ${KEY_FILE}"
echo "Cert file: ${CERT_FILE}"
