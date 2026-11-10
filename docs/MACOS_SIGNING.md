# macOS signing and notarization

Release binaries for macOS are signed with a **Developer ID Application** certificate
and notarized with `xcrun notarytool`. A stable signing identity keeps Full Disk Access
and Automation grants across upgrades.

Script: `scripts/macos-sign-notarize.sh`. Releases call it from `.github/workflows/release.yml`.
Test it any time with the manual **Sign Test** workflow (`sign-test.yml`).

## GitHub secrets

| Secret | Value |
|---|---|
| `MACOS_CERT_P12_BASE64` | base64 of the exported Developer ID `.p12` |
| `MACOS_CERT_PASSWORD` | password set when exporting the `.p12` |
| `APPLE_TEAM_ID` | Team ID (`YR2436ED8U`) |
| `NOTARY_API_KEY_ID` | App Store Connect API key ID |
| `NOTARY_API_ISSUER_ID` | App Store Connect issuer ID |
| `NOTARY_API_KEY_P8` | contents of the `AuthKey_*.p8` file |

## CI keychain handling

The script creates a temporary keychain on the runner, imports the `.p12`, signs,
and deletes the keychain on exit (including on failure). Nothing persists on the runner.

## Setup from scratch

1. Create a CSR in Keychain Access and request a **Developer ID Application** cert at
   developer.apple.com (Account Holder only).
2. Install the `.cer`. In Keychain Access, **My Certificates**, check the private key appears
   under the cert, then export as `.p12`.
3. Create an App Store Connect API key (Users and Access, Integrations) with the
   Developer role. The `.p8` downloads only once.
4. `gh secret set` each secret above. Delete the local `.p12` and `.p8` afterwards.

## Rotation and expiry

- **Developer ID cert expires Dec 10, 2027.** Create a new cert and update
  `MACOS_CERT_P12_BASE64` and `MACOS_CERT_PASSWORD` before then. A new identity
  means users may need to re-grant Full Disk Access once.
- **Apple Developer Program membership** renews yearly. If it lapses, notarization fails.
- **API key** does not expire; revoke and replace it if leaked.
- Rotate secrets by re-running the setup steps; no workflow change needed.

## Stapling

Bare Mach-O binaries cannot be stapled (`stapler` supports only `.app`, `.dmg`, `.pkg`).
The binary is notarized and Gatekeeper retrieves the ticket online on first run.
`spctl --assess` also rejects bare binaries even when notarized. To get an offline-valid,
`stapler validate`-passing artifact, ship a `.dmg` or `.pkg` instead.
