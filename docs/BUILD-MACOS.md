# Building PBR Studio for macOS (DMG)

The macOS installer is built as a DMG (Apple Disk Image). **DMG can only be built on macOS** because it requires Apple's build tools.

## Prerequisites

### 1. Node.js and npm

Install Node.js LTS (v18 or newer). Using [Homebrew](https://brew.sh):

```bash
brew install node
```

### 2. Rust

Install Rust via [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 3. Xcode Command Line Tools

Install the Xcode Command Line Tools (required for building):

```bash
xcode-select --install
```

For full Xcode (optional, for advanced development):

```bash
xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

## Build

### Option A: DMG only

From `pbr-studio-ui`:

```bash
npm ci
npm run dmg
```

Output: `src-tauri/target/release/bundle/dmg/pbr-studio-ui_1.0.0_aarch64.dmg` (Apple Silicon) or `pbr-studio-ui_1.0.0_x64.dmg` (Intel).

### Option B: Full build (all formats)

```bash
npm run tauri:build
```

This produces both `.app` and `.dmg` bundles.

### Option C: Build for specific architecture

```bash
# Apple Silicon (M1/M2/M3)
npm run tauri build -- --target aarch64-apple-darwin

# Intel Mac
npm run tauri build -- --target x86_64-apple-darwin

# Universal binary (both architectures)
npm run tauri build -- --target universal-apple-darwin
```

### Option D: GitHub Actions (tag-based)

Push a version tag (e.g. `v1.0.0`) to trigger cross-platform builds including macOS DMG. See [CI-RELEASE.md](CI-RELEASE.md).

## Configuration

DMG settings are in `tauri.conf.json` under `bundle.macOS.dmg`:

- **windowSize**: 660×400 (default installer window size)
- **appPosition**: Icon position for the app
- **applicationFolderPosition**: Icon position for the Applications folder
- **background**: Custom background image (optional)

## Output location

DMG files are written to:

```
pbr-studio-ui/src-tauri/target/release/bundle/dmg/
```

## Code signing and notarization

For distribution outside the App Store, Apple recommends signing and notarizing your app. See [Tauri's macOS signing guide](https://tauri.app/distribute/sign/macos/) for details.

---

## macOS Gatekeeper and code signing

Gatekeeper is macOS's security feature that blocks unsigned or unnotarized apps from running. To avoid "app is damaged" or "cannot be opened" warnings for users:

### 1. Ad-hoc signing (minimal, no Apple Developer account)

Sign the app with an ad-hoc identity so it runs on your Mac without quarantine:

```bash
codesign --force --deep --sign - \
  pbr-studio-ui/src-tauri/target/release/bundle/macos/PBR\ Studio.app
```

For the DMG, sign the app before creating the DMG, or sign the DMG itself:

```bash
codesign --force --sign - pbr-studio-ui_1.0.0_aarch64.dmg
```

**Limitation**: Ad-hoc signing does not satisfy Gatekeeper for distribution. Users who download the DMG will still see a warning unless they right-click → Open, or you use Developer ID signing.

### 2. Developer ID signing (recommended for distribution)

Requires an [Apple Developer account](https://developer.apple.com) ($99/year).

1. **Create a Developer ID Application certificate** in [Certificates, Identifiers & Profiles](https://developer.apple.com/account/resources/certificates/list):
   - Certificates → + → Developer ID Application

2. **Configure Tauri** in `tauri.conf.json` (or via environment):

   ```json
   {
     "bundle": {
       "macOS": {
         "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)"
       }
     }
   }
   ```

   Or set at build time:
   ```bash
   export TAURI_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
   npm run tauri build -- --bundles dmg
   ```

3. **Sign the app** (Tauri may do this automatically when configured):
   ```bash
   codesign --force --deep --options runtime \
     --sign "Developer ID Application: Your Name (TEAM_ID)" \
     "PBR Studio.app"
   ```

### 3. Notarization (required for Gatekeeper to allow without user override)

After signing with Developer ID, submit for notarization:

```bash
# Create a notarization key in App Store Connect (Users and Access → Keys)
# Then:
xcrun notarytool submit pbr-studio-ui_1.0.0_aarch64.dmg \
  --apple-id "your@email.com" \
  --team-id "TEAM_ID" \
  --password "@keychain:AC_PASSWORD" \
  --wait

# Staple the notarization ticket to the DMG
xcrun stapler staple pbr-studio-ui_1.0.0_aarch64.dmg
```

Users can then open the DMG and run the app without security warnings.

### 4. CI/CD integration

Set these secrets in GitHub Actions (or your CI):

- `APPLE_CERTIFICATE` – Base64-encoded `.p12` certificate
- `APPLE_CERTIFICATE_PASSWORD` – Password for the certificate
- `APPLE_SIGNING_IDENTITY` – e.g. `"Developer ID Application: ..."`
- `APPLE_ID` – Apple ID for notarization
- `APPLE_APP_SPECIFIC_PASSWORD` – App-specific password (not your main Apple ID password)

See [Tauri's macOS signing guide](https://tauri.app/distribute/sign/macos/) for full CI setup.
