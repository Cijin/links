#!/bin/sh
# Builds Links and wraps it in a macOS app bundle at dist/Links.app.
# Drag it into /Applications, or run with --install to copy it to ~/Applications.
set -eu
cd "$(dirname "$0")/.."

cargo build --release

APP=dist/Links.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/links "$APP/Contents/MacOS/Links"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Links</string>
  <key>CFBundleDisplayName</key><string>Links</string>
  <key>CFBundleIdentifier</key><string>me.local.links</string>
  <key>CFBundleExecutable</key><string>Links</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
</dict>
</plist>
PLIST
codesign --force --sign - "$APP" >/dev/null 2>&1 || echo "codesign skipped"
echo "Built $APP"

if [ "${1:-}" = "--install" ]; then
  mkdir -p "$HOME/Applications"
  rm -rf "$HOME/Applications/Links.app"
  cp -R "$APP" "$HOME/Applications/Links.app"
  echo "Installed ~/Applications/Links.app"
fi
