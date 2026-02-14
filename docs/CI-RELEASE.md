# CI & Release Workflow

The `.github/workflows/ci-release.yml` workflow runs on tag pushes (`v*`) and manual dispatch.

## Jobs

### 1. CLI batch check

- Creates minimal material fixtures in `ci/fixtures/sample-material/`
- Builds `pbr-cli` and runs `batch-check` in CI mode
- Outputs structured JSON for pipeline integration

### 2. Report generation

- Generates HTML and PDF reports from fixtures
- Requires `fonts-liberation` for PDF
- Uploads reports as artifacts

### 3. Cross-platform builds

- **Linux**: AppImage
- **Windows**: MSI
- **macOS**: DMG

### 4. Enterprise artifact storage (optional)

Enable by setting these repository secrets:

- `ARTIFACT_STORAGE_URL` – Base URL for your artifact store
- `ARTIFACT_STORAGE_TOKEN` (optional) – Bearer token for auth

When set, the job downloads all artifacts and prints instructions for uploading to S3, Artifactory, Azure Blob, etc. Customize the upload step for your stack.

### 5. GitHub Release

On tag push, creates a draft release with all build artifacts and reports attached.

## Usage

```bash
# Tag and push to trigger
git tag v0.2.0
git push origin v0.2.0
```

Or run manually: **Actions → CI & Release → Run workflow**
