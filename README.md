# guidebook

A fast, HonKit/GitBook-compatible static site generator written in Rust.

## Features

- **GitBook/HonKit Compatible** - Drop-in replacement for GitBook/HonKit projects
- **Fast** - Built with Rust for maximum performance
- **Hot Reload** - Live preview server with automatic rebuild on file changes
- **Multi-language Support** - Build books in multiple languages with `LANGS.md`
- **Mermaid Diagrams** - Native support for Mermaid diagram rendering
- **Collapsible Chapters** - Sidebar with expandable/collapsible chapter navigation
- **Page Navigation** - Previous/Next page arrows for easy navigation
- **SPA Navigation** - Smooth page transitions without full page reloads
- **Japanese Support** - Proper handling of Japanese characters in heading IDs and anchors
- **Full-width Space Tolerance** - Automatically handles full-width spaces in Markdown headings
- **Custom Styles** - Load custom CSS from `styles/website.css`
- **Auto-link URLs** - Bare URLs are automatically converted to clickable links (except in code blocks)
- **Sidebar State Persistence** - Sidebar open/close state is saved in localStorage

## Installation

### Step 1: Install Rust

**Mac / Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:**

Download and run the installer from https://rustup.rs/

After installation, restart your terminal.

### Step 2: Install guidebook

```bash
cargo install guidebook
```

### Step 3: Verify Installation

```bash
guidebook --version
```

## Quick Start

```bash
# Navigate to your book folder
cd your-book-folder

# Start preview server with hot reload
guidebook serve

# Open http://localhost:4000 in your browser
```

### From Source (for developers)

```bash
git clone https://github.com/guide-inc-org/guidebook.git
cd guidebook
cargo build --release
```

The binary will be available at `target/release/guidebook`.

## Usage

```bash
# Build a book
guidebook build <source-directory> -o <output-directory>

# Start development server with hot reload
guidebook serve <source-directory> -p <port>

# Examples
guidebook build ./my-book -o ./dist
guidebook serve ./my-book -p 4000
```

### Serve Command (Hot Reload)

The `serve` command starts a local development server with hot reload:

```bash
guidebook serve ./my-book -p 4000
```

```
📚 Serving book at http://localhost:4000/
   🔥 Hot reload enabled - changes will auto-refresh
   Press Ctrl+C to stop
```

**Features:**
- Watches source files (`.md`, `.json`, `.css`, `.js`) for changes
- Automatically rebuilds when files are modified
- Browser auto-refreshes after rebuild (1 second polling)
- Previous/Next navigation arrows on each page

## Project Structure

guidebook expects a GitBook/HonKit-compatible project structure:

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
  "styles": {
    "website": "styles/website.css"
  }
}
```

### Custom Styles (styles/website.css)

You can customize fonts and styles by creating `styles/website.css`:

```css
/* Google Fonts for Japanese */
@import url(https://fonts.googleapis.com/css?family=Noto+Sans+JP|Noto+Serif+JP|Roboto+Mono&display=swap&subset=japanese);

/* Apply Noto Sans JP to the book */
.book.font-family-1 {
    font-family: "Noto Sans JP", "メイリオ", sans-serif;
}

/* Custom heading styles */
.markdown-section h2 {
    border-left: 7px solid rgb(16, 122, 126);
    padding-left: 10px;
    background-color: rgb(244, 244, 244);
}

/* Code font */
.markdown-section pre,
.markdown-section code {
    font-family: "Roboto Mono", Consolas, monospace;
}
```

The CSS file path is configured in `book.json` under `styles.website`.

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
| Auto-link URLs | ✅ |
| Sidebar state persistence | ✅ |
| Image paths with spaces | ✅ |
| Hot reload (serve command) | ✅ |
| Page navigation (prev/next) | ✅ |

## Development

```bash
# Run in development
cargo run -- build <source> -o <output>

# Run dev server with hot reload
cargo run -- serve <source> -p 4000

# Run tests
cargo test

# Build release
cargo build --release
```

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
