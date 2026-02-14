# Building PBR Studio as AppImage (Linux)

AppImage is a portable Linux format that bundles all dependencies. Users can run it without installation by making it executable and double-clicking or running it from the terminal.

## Prerequisites

### 1. Node.js and npm

Install Node.js LTS (v18 or newer). Check with:

```bash
node -v
npm -v
```

### 2. Rust

Install Rust via [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 3. Linux system dependencies

Tauri requires WebKit and related libraries. On **Ubuntu/Debian**:

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libfuse2
```

On **Fedora**:

```bash
sudo dnf install webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel patchelf fuse
```

On **Arch Linux**:

```bash
sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3 librsvg patchelf fuse2
```

## Build methods

### Option A: Build script (recommended)

From the project root:

```bash
chmod +x scripts/build-appimage.sh
./scripts/build-appimage.sh
```

The AppImage will be created at:

```
pbr-studio-ui/src-tauri/target/release/bundle/appimage/pbr-studio-ui_<version>_amd64.AppImage
```

### Option B: Manual build

```bash
cd pbr-studio-ui
npm ci
npm run tauri build -- --bundles appimage --ci
```

### Option C: GitHub Actions

- **Tag-based release**: Push a version tag (e.g. `v1.0.0`) to trigger cross-platform builds (Linux AppImage, Windows MSI, macOS DMG). See [`.github/workflows/ci-release.yml`](../.github/workflows/ci-release.yml).

## Running the AppImage

```bash
chmod +x pbr-studio-ui_1.0.0_amd64.AppImage
./pbr-studio-ui_1.0.0_amd64.AppImage
```

Or double-click it in a file manager (ensure it has execute permission).

## Distribution notes

- **glibc compatibility**: Build on the oldest Linux you want to support. A binary built on Ubuntu 22.04 may fail on older systems with errors like `GLIBC_2.33 not found`. Use Ubuntu 20.04 or a Docker container for broader compatibility.
- **Size**: AppImage bundles are typically 70+ MB due to bundled libraries.
- **FUSE**: Running AppImage requires `libfuse2`. Some newer systems ship only `libfuse3`; install `libfuse2` if needed.

---

## Validating minimum glibc version

The AppImage built by CI uses **Ubuntu 22.04**, which provides **glibc 2.35**. The binary therefore requires **glibc 2.35 or newer** on the target system.

### Check system glibc version

On the target Linux system:

```bash
ldd --version
```

Example output: `ldd (Ubuntu GLIBC 2.35-0ubuntu3.1) 2.35` → minimum is 2.35.

### Check minimum glibc required by the AppImage

To validate which glibc versions the built AppImage requires:

1. **Extract the AppImage** (creates `squashfs-root/`):

   ```bash
   ./pbr-studio-ui_1.0.0_amd64.AppImage --appimage-extract
   ```

2. **Find the main binary** (Tauri app executable under `squashfs-root/`):

   ```bash
   find squashfs-root -type f -executable ! -name "*.so*" | head -5
   ```

3. **List required glibc versions** (replace `BINARY_PATH` with the path from step 2):

   ```bash
   objdump -T BINARY_PATH | grep GLIBC | \
     sed 's/.*GLIBC_\([.0-9]*\).*/\1/' | sort -Vu
   ```

   The **highest version** in the output is the minimum glibc required.

### glibc by distribution

| Distribution | glibc | Compatible with Ubuntu 22.04 build? |
|--------------|-------|-------------------------------------|
| Ubuntu 24.04 | 2.39  | Yes                                 |
| Ubuntu 22.04 | 2.35  | Yes                                 |
| Ubuntu 20.04 | 2.31  | No (upgrade glibc or rebuild)       |
| Debian 11    | 2.31  | No                                  |
| Debian 12    | 2.36  | Yes                                 |
| Fedora 36+   | 2.35+ | Yes                                 |

### Building for broader compatibility

To support Ubuntu 20.04 / Debian 11 (glibc 2.31+), build on an older base:

- Use **Ubuntu 20.04** in CI: change `runs-on: ubuntu-22.04` to `ubuntu-20.04` for the AppImage job.
- Or use a Docker image: `docker run -v $(pwd):/app -w /app ubuntu:20.04 ./scripts/build-appimage.sh`
