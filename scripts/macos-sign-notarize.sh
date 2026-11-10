#!/usr/bin/env bash
# Sign a macOS binary with the Developer ID cert and notarize it.
# Usage: scripts/macos-sign-notarize.sh <binary>
# Env: MACOS_CERT_P12_BASE64, MACOS_CERT_PASSWORD, APPLE_TEAM_ID,
#      NOTARY_API_KEY_ID, NOTARY_API_ISSUER_ID, NOTARY_API_KEY_P8
set -euo pipefail

bin="$1"
tmp="$(mktemp -d)"
kc="$tmp/signing.keychain-db"
kc_pw="$(uuidgen)"
original_keychains="$(security list-keychains -d user | tr -d '"' | xargs)"

cleanup() {
  # shellcheck disable=SC2086
  security list-keychains -d user -s $original_keychains || true
  security delete-keychain "$kc" 2>/dev/null || true
  rm -rf "$tmp"
}
trap cleanup EXIT

echo "$MACOS_CERT_P12_BASE64" | base64 --decode > "$tmp/cert.p12"
security create-keychain -p "$kc_pw" "$kc"
security set-keychain-settings -lut 3600 "$kc"
security unlock-keychain -p "$kc_pw" "$kc"
security import "$tmp/cert.p12" -k "$kc" -P "$MACOS_CERT_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: -s -k "$kc_pw" "$kc" >/dev/null
# shellcheck disable=SC2086
security list-keychains -d user -s "$kc" $original_keychains

identity="$(security find-identity -v -p codesigning "$kc" | grep -o 'Developer ID Application: .*(.*)' | head -1 | tr -d '"')"
[ -n "$identity" ] || { echo "No Developer ID Application identity found" >&2; exit 1; }
case "$identity" in *"($APPLE_TEAM_ID)") ;; *) echo "Identity team mismatch: $identity" >&2; exit 1 ;; esac

codesign --force --options runtime --timestamp --sign "$identity" "$bin"
codesign --verify --strict --verbose=2 "$bin"

echo "$NOTARY_API_KEY_P8" > "$tmp/key.p8"
ditto -c -k --keepParent "$bin" "$tmp/notarize.zip"
out="$(xcrun notarytool submit "$tmp/notarize.zip" \
  --key "$tmp/key.p8" --key-id "$NOTARY_API_KEY_ID" --issuer "$NOTARY_API_ISSUER_ID" \
  --wait --timeout 30m --output-format json)"
echo "$out"
echo "$out" | jq -e '.status == "Accepted"' >/dev/null \
  || { echo "Notarization not accepted" >&2; exit 1; }

# Bare Mach-O binaries cannot be stapled; Gatekeeper checks the ticket online.
echo "Signed and notarized: $bin"
