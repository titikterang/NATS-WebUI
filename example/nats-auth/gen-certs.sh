#!/usr/bin/env bash
# Generates a throwaway local CA and a server certificate for the nats-auth example.
# Output: example/nats-auth/certs/ (git-ignored). For local testing only.
#
# SANs cover how clients reach the server: localhost / 127.0.0.1 from the host, and nats-auth
# from containers on the nats-examples network.
set -euo pipefail
cd "$(dirname "$0")"
dir=certs
if [[ -f "$dir/ca.pem" && -f "$dir/server-cert.pem" ]]; then
  echo "certs exist in $(pwd)/$dir (delete the directory to regenerate)"
  exit 0
fi
mkdir -p "$dir"

openssl req -x509 -newkey rsa:3072 -nodes -days 365 -subj "/CN=nats-example-ca" \
  -addext "basicConstraints=critical,CA:TRUE" \
  -addext "keyUsage=critical,keyCertSign,cRLSign" \
  -addext "subjectKeyIdentifier=hash" \
  -keyout "$dir/ca-key.pem" -out "$dir/ca.pem" 2>/dev/null

openssl req -newkey rsa:2048 -nodes -subj "/CN=nats-auth" \
  -keyout "$dir/server-key.pem" -out "$dir/server.csr" 2>/dev/null

openssl x509 -req -in "$dir/server.csr" -CA "$dir/ca.pem" -CAkey "$dir/ca-key.pem" -CAcreateserial \
  -days 365 -out "$dir/server-cert.pem" \
  -extfile <(printf '%s\n' \
    'subjectAltName=DNS:localhost,DNS:nats-auth,IP:127.0.0.1' \
    'basicConstraints=critical,CA:FALSE' \
    'keyUsage=critical,digitalSignature,keyEncipherment' \
    'extendedKeyUsage=serverAuth' \
    'subjectKeyIdentifier=hash' \
    'authorityKeyIdentifier=keyid,issuer') 2>/dev/null
# The extensions above are required by strict verifiers (Python 3.13+ enables X509_STRICT).

rm -f "$dir/server.csr" "$dir/ca.srl"
# The nats container runs as root; the key only needs to be readable inside it.
chmod 644 "$dir/server-key.pem"
echo "created $(pwd)/$dir: ca.pem (give this to clients), server-cert.pem, server-key.pem"
