# CLAUDE.md - guidebook

A static site generator compatible with HonKit/GitBook.

## Project Overview

- **Language:** Rust
- **Published to:** crates.io (`cargo install guidebook`)
- **Repository:** https://github.com/guide-inc-org/guidebook

## Build & Test

```bash
# Build
cargo build --release

# Test
cargo test

# Build documentation locally
./target/release/guidebook build

# Start dev server
./target/release/guidebook serve
```

## Release Procedure

### Quick Release (GitHub Releases only)

For most releases, just push a tag. GitHub Actions will automatically build binaries for all platforms and create a GitHub Release.

```bash
# 1. Update version in Cargo.toml
# 2. Update Cargo.lock
cargo check

# 3. Commit and push
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to X.Y.Z"
git push origin main

# 4. Create and push tag (triggers GitHub Actions)
git tag vX.Y.Z
git push origin vX.Y.Z
```

The release workflow will:
- Build binaries for Linux, macOS (Intel + Apple Silicon), and Windows
- Create a GitHub Release with all binaries attached
- Guidebook Cloud will automatically pick up the new version on next build

### Full Release (GitHub + crates.io)

If you also want to publish to crates.io:

```bash
# After pushing the tag, publish to crates.io
cargo publish
```

### Notes

- Always run `cargo check` after updating version to ensure `Cargo.lock` is updated
- Guidebook Cloud auto-updates: The builder checks GitHub Releases on each build and downloads the latest version if newer
- No need to rebuild the Fly.io builder image when releasing new versions

## Directory Structure

```
src/
├── main.rs          # CLI entry point
├── builder/
│   ├── mod.rs       # Build process
│   ├── renderer.rs  # Markdown to HTML conversion
│   └── template.rs  # HTML template
├── parser/
│   ├── mod.rs
│   ├── book_config.rs  # book.json parser
│   ├── langs.rs        # LANGS.md parser (multi-language support)
│   └── summary.rs      # SUMMARY.md parser
templates/
├── gitbook.css      # Stylesheet
├── gitbook.js       # Client-side JS
├── collapsible.js   # Collapsible sections
└── search.js        # Search functionality
```

## Important Design Decisions

### Do NOT use `<base>` tag

**Reason:** Using `<base href>` causes relative image paths in markdown (e.g., `../../../assets/...`) to resolve from base, breaking when deployed to subdirectories.

**Solution:** Embed `root_path` directly into CSS/JS/links (same approach as HonKit)

## CI/CD

### Release Workflow

`.github/workflows/release.yml` - Publishes multi-platform binaries to GitHub Releases on tag push.

**Supported platforms:**
- Linux x86_64
- macOS x86_64 (Intel)
- macOS arm64 (Apple Silicon)
- Windows x86_64

**Installation (no Rust required):**
```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/guide-inc-org/guidebook/main/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/guide-inc-org/guidebook/main/install.ps1 | iex

# Update to latest version
guidebook update
```

**CI pipeline usage:**
```yaml
- name: Install guidebook
  run: |
    curl -sL https://github.com/guide-inc-org/guidebook/releases/latest/download/guidebook-linux-x86_64.tar.gz | tar xz
    ./guidebook build
```

## TODO

- [x] Update README.md
  - Add quick install instructions (curl | sh)
  - Add `guidebook update` command documentation
  - Simplify structure (refer to HonKit's README)
- [x] Create `docs/` folder (multi-language: English, Japanese & Vietnamese)
  - LANGS.md - Language selection
  - en/ - English documentation
    - README.md - Introduction
    - SUMMARY.md - Table of contents
    - installation.md - Installation guide
    - quick-start.md - Quick start guide
    - config.md - book.json configuration
    - structure.md - Project structure
    - features/ - Feature documentation (mermaid, collapsible, search)
    - migration.md - Migration from HonKit
    - faq.md - FAQ
  - ja/ - Japanese documentation (same structure)
  - vi/ - Vietnamese documentation (same structure)
- [x] Build docs with guidebook and publish to GitHub Pages
- [x] Add GitHub Pages documentation link to README.md

## Changelog

- **2025-12-25 v0.1.13:** Enable collapsible.js by default (no book.json required)
- **2025-12-25 v0.1.12:** Fix SPA navigation URL accumulation bug
- **2025-12-25 v0.1.10:** Fix image paths (remove `<base>` tag), add release workflow
