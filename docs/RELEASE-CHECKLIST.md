# Release Checklist

Pre-release steps for publishing PBR Studio. Use this checklist before tagging a version or publishing builds.

---

## Pre-release verification

### 1. Version and changelog

- [ ] Update `version` in `pbr-studio-ui/package.json` and `pbr-studio-ui/src-tauri/tauri.conf.json`
- [ ] Update `version` in `pbr-core/Cargo.toml` and `pbr-cli` if applicable
- [ ] Add release notes or update `CHANGELOG.md` (if present)

### 2. Build verification

- [ ] **CLI**: `cargo build -p pbr-cli --release` succeeds
- [ ] **CLI tests**: `cargo test -p pbr-core` passes
- [ ] **UI**: `cd pbr-studio-ui && npm run build` succeeds
- [ ] **Tauri**: `cd pbr-studio-ui && npm run tauri build` produces all target bundles

### 3. Functional smoke tests

- [ ] CLI `check`, `batch-check`, `export-report`, `analyze` run without errors
- [ ] UI launches, loads material folders, shows validation, exports report
- [ ] No console errors or network calls in production build

---

## Platform-specific hardening

### Linux AppImage

- [ ] Build completes: `./scripts/build-appimage.sh` or CI
- [ ] **glibc validation**: AppImage runs on target distros (see [BUILD-APPIMAGE.md](BUILD-APPIMAGE.md#validating-minimum-glibc-version))
  - CI builds on Ubuntu 22.04 → requires glibc 2.35+
  - For Ubuntu 20.04 / Debian 11 support, build on older base
- [ ] Test on a clean system: `chmod +x *.AppImage && ./pbr-studio-ui_*.AppImage`
- [ ] Verify `libfuse2` requirement is documented for users

### macOS DMG

- [ ] Build completes on macOS (or via CI)
- [ ] **Gatekeeper / code signing**: See [BUILD-MACOS.md](BUILD-MACOS.md#macos-gatekeeper-and-code-signing)
  - Ad-hoc signing for local use: `codesign --force --deep --sign - "PBR Studio.app"`
  - Developer ID + notarization for distribution (no Gatekeeper warnings)
- [ ] Test DMG on a clean Mac: open, drag to Applications, run
- [ ] If distributing: notarize and staple the DMG

### Windows MSI

- [ ] Build completes on Windows (or via CI)
- [ ] Installer runs and installs correctly
- [ ] App launches from Start Menu / desktop shortcut

---

## CI & release workflow

- [ ] Push tag to trigger release: `git tag v0.2.0 && git push origin v0.2.0`
- [ ] Or run manually: **Actions → CI & Release → Run workflow**
- [ ] Verify all jobs pass: CLI batch check, reports, Linux AppImage, Windows MSI, macOS DMG
- [ ] Review draft GitHub Release; add release notes; publish when ready

---

## Post-release

- [ ] Verify download links work
- [ ] Update README or docs with new version if needed
- [ ] Announce release (internal, changelog, etc.)

---

## Quick reference

| Doc | Purpose |
|-----|---------|
| [BUILD-APPIMAGE.md](BUILD-APPIMAGE.md) | Linux build, glibc validation |
| [BUILD-MACOS.md](BUILD-MACOS.md) | macOS build, Gatekeeper, code signing |
| [BUILD-WINDOWS.md](BUILD-WINDOWS.md) | Windows build |
| [CI-RELEASE.md](CI-RELEASE.md) | CI workflow, tag-based releases |
| [PRODUCTION-CHECKLIST.md](PRODUCTION-CHECKLIST.md) | Full feature/implementation checklist |
