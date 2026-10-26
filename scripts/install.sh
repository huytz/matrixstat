#!/bin/sh
# Build from source, install ~/Applications/matrixstat.app, optionally register a LaunchAgent.
set -eu
cd "$(dirname "$0")/.."
cargo build --release
app="$HOME/Applications/matrixstat.app"
mkdir -p "$HOME/Applications"
scripts/bundle.sh target/release/matrixstat "$app"
if [ "${1:-}" = "--launchd" ]; then
  plist="$HOME/Library/LaunchAgents/dev.huytz.matrixstat.plist"
  mkdir -p "$(dirname "$plist")"
  cat > "$plist" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>dev.huytz.matrixstat</string>
  <key>ProgramArguments</key><array><string>$app/Contents/MacOS/matrixstat</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict></plist>
PL
  launchctl bootout "gui/$(id -u)/dev.huytz.matrixstat" 2>/dev/null || true
  launchctl bootstrap "gui/$(id -u)" "$plist"
  echo "launch agent loaded: $plist"
else
  echo "run: open $app   (add --launchd to start at login)"
fi
