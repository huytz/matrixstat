#!/bin/sh
# Wrap a built binary into an ad-hoc signed matrixstat.app: bundle.sh <binary> <out.app>
set -eu
bin=${1:?binary path}
app=${2:?output .app path}
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$(dirname "$0")/../Cargo.toml" | head -1)
rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp "$bin" "$app/Contents/MacOS/matrixstat"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>dev.huytz.matrixstat</string>
  <key>CFBundleName</key><string>matrixstat</string>
  <key>CFBundleExecutable</key><string>matrixstat</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version:-0.0.0}</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
# Ad-hoc signature so Gatekeeper and launchd accept the bundle on Apple Silicon.
codesign --force --sign - "$app" >/dev/null
echo "bundled $app"
