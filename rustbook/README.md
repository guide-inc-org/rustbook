# rustbook

A fast, HonKit/GitBook-compatible static site generator written in Rust.

## Features

- **GitBook/HonKit Compatible** - Drop-in replacement for GitBook/HonKit projects
- **Fast** - Built with Rust for maximum performance
- **Multi-language Support** - Build books in multiple languages with `LANGS.md`
- **Mermaid Diagrams** - Native support for Mermaid diagram rendering
- **Collapsible Chapters** - Sidebar with expandable/collapsible chapter navigation
- **SPA Navigation** - Smooth page transitions without full page reloads
- **Japanese Support** - Proper handling of Japanese characters in heading IDs and anchors
- **Full-width Space Tolerance** - Automatically handles full-width spaces in Markdown headings

## Installation

### From Source

```bash
git clone https://github.com/guide-inc-org/rustbook.git
cd rustbook/rustbook
cargo build --release
```

The binary will be available at `target/release/rustbook`.

## Usage

```bash
# Build a book
rustbook build <source-directory> -o <output-directory>

# Example
rustbook build ./my-book -o ./dist
```

## Project Structure

rustbook expects a GitBook/HonKit-compatible project structure:

```
my-book/
├── book.json          # Book configuration
├── README.md          # Book introduction (becomes index.html)
├── SUMMARY.md         # Table of contents
├── LANGS.md           # (Optional) Multi-language configuration
├── chapter1.md
├── chapter2/
│   ├── section1.md
│   └── section2.md
└── assets/
    └── images/
```

### book.json

```json
{
  "title": "My Book",
  "plugins": ["collapsible-chapters"],
  "pluginsConfig": {
    "theme-default": {
      "styles": {
        "website": "styles/website.css"
      }
    }
  }
}
```

### SUMMARY.md

```markdown
# Summary

* [Introduction](README.md)
* [Chapter 1](chapter1.md)
* [Chapter 2](chapter2/README.md)
  * [Section 1](chapter2/section1.md)
  * [Section 2](chapter2/section2.md)
```

### LANGS.md (Multi-language)

```markdown
# Languages

* [English](en/)
* [Japanese](jp/)
```

## Supported Features

| Feature | Status |
|---------|--------|
| Markdown rendering | ✅ |
| Tables | ✅ |
| Code blocks with syntax highlighting | ✅ |
| Mermaid diagrams | ✅ |
| Task lists | ✅ |
| Footnotes | ✅ |
| Strikethrough | ✅ |
| Collapsible chapters | ✅ |
| Multi-language books | ✅ |
| Custom styles | ✅ |
| Anchor links with Japanese text | ✅ |
| SPA-like navigation | ✅ |

## Development

```bash
# Run in development
cargo run -- build <source> -o <output>

# Run tests
cargo test

# Build release
cargo build --release
```

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
