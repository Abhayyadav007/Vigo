#!/usr/bin/env bash
# Writes placeholder Firebase config files for the offline `demo-vigo` project,
# so dev builds boot on simulators/emulators against the Firebase Auth emulator
# (`pnpm emulators`). Never ship these: for real devices and the stores, replace
# them with the files from your Firebase project (see apps/mobile-*/firebase/README.md).
set -euo pipefail
cd "$(dirname "$0")/.."

for app in customer picker rider; do
  bundle="com.vigo.$app"
  dir="apps/mobile-$app/firebase"
  mkdir -p "$dir"
  [[ -e "$dir/GoogleService-Info.plist" ]] || cat > "$dir/GoogleService-Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>API_KEY</key><string>demo-api-key</string>
  <key>GCM_SENDER_ID</key><string>123456789012</string>
  <key>PLIST_VERSION</key><string>1</string>
  <key>BUNDLE_ID</key><string>$bundle</string>
  <key>PROJECT_ID</key><string>demo-vigo</string>
  <key>STORAGE_BUCKET</key><string>demo-vigo.appspot.com</string>
  <key>GOOGLE_APP_ID</key><string>1:123456789012:ios:0000000000000000000000</string>
  <key>CLIENT_ID</key><string>123456789012-demo.apps.googleusercontent.com</string>
  <key>REVERSED_CLIENT_ID</key><string>com.googleusercontent.apps.123456789012-demo</string>
</dict>
</plist>
PLIST
  [[ -e "$dir/google-services.json" ]] || cat > "$dir/google-services.json" <<JSON
{
  "project_info": { "project_number": "123456789012", "project_id": "demo-vigo", "storage_bucket": "demo-vigo.appspot.com" },
  "client": [{
    "client_info": {
      "mobilesdk_app_id": "1:123456789012:android:0000000000000000000000",
      "android_client_info": { "package_name": "$bundle" }
    },
    "api_key": [{ "current_key": "demo-api-key" }],
    "oauth_client": [],
    "services": { "appinvite_service": { "other_platform_oauth_client": [] } }
  }],
  "configuration_version": "1"
}
JSON
  echo "firebase dev files ready in $dir"
done
