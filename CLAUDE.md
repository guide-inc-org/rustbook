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

1. Update version in `Cargo.toml`
2. Commit & push
3. Create and push tag (binary is auto-generated to GitHub Releases)
4. Publish to crates.io

```bash
# After version update
git add -A && git commit -m "Bump version to vX.Y.Z"
git push origin main

# Create & push tag (triggers release workflow)
git tag vX.Y.Z
git push origin vX.Y.Z

# Publish to crates.io
cargo publish
```

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

`.github/workflows/release.yml` - Publishes Linux binary to GitHub Releases on tag push.

Consumers download pre-built binary:
```yaml
- name: Install guidebook
  run: |
    curl -sL https://github.com/guide-inc-org/guidebook/releases/latest/download/guidebook-linux-x86_64.tar.gz | tar xz
    ./guidebook build
```

## Changelog

- **2025-12-25 v0.1.12:** Fix SPA navigation URL accumulation bug
- **2025-12-25 v0.1.10:** Fix image paths (remove `<base>` tag), add release workflow
