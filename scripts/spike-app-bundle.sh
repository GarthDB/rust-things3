#!/usr/bin/env bash
# Throwaway Tasklink.app for the #228 permissions spike. Not the release packager (#227).
# Usage: scripts/spike-app-bundle.sh <binary> [bundle-id]   Env: SIGN_ID (default: ad-hoc "-")
set -euo pipefail

bin="$1"
id="${2:-com.garthdb.tasklink.spike$(date +%s)}"
app="${OUT:-$PWD/spike/Tasklink.app}"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp "$bin" "$app/Contents/MacOS/tasklink"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>$id</string>
  <key>CFBundleName</key><string>Tasklink</string>
  <key>CFBundleExecutable</key><string>tasklink</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.0.0-spike</string>
  <key>LSUIElement</key><true/>
  <key>NSAppleEventsUsageDescription</key><string>Tasklink needs to control Things 3 to create and update your to-dos.</string>
</dict></plist>
PLIST
codesign --force --options runtime --timestamp=none --sign "${SIGN_ID:--}" "$app"
codesign --verify --deep --strict "$app"
echo "Bundle: $app"
echo "ID:     $id"
echo "Run:    $app/Contents/MacOS/tasklink mcp"
echo "Reset:  tccutil reset All $id"
