Put this app's Firebase config files here (gitignored):

- `GoogleService-Info.plist` (iOS)
- `google-services.json` (Android)

Download them from Firebase console → Project settings → Your apps, for the
bundle ID / package in `app.json`. Or point `GOOGLE_SERVICE_INFO_PLIST` /
`GOOGLE_SERVICES_JSON` at them. Then run `pnpm prebuild` and `pnpm ios|android`.
