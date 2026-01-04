# FAQ

## Installation

### Do I need Rust to use guidebook?

No. Pre-built binaries are available for macOS, Linux, and Windows. Just run the install script.

### How do I update guidebook?

```bash
guidebook update
```

## Usage

### Where is the output?

By default, `guidebook build` outputs to `_book/`. You can change this with `-o`:

```bash
guidebook build -o dist
```

### How do I change the port?

```bash
guidebook serve -p 3000
```

### Why isn't search working in development?

The search index is not regenerated on hot reload to improve performance. Restart `guidebook serve` to update the search index.

## Compatibility

### Does it work with my HonKit project?

Yes, guidebook is a drop-in replacement. Just run `guidebook build` instead of `npx honkit build`.

### Can I use JavaScript plugins?

No, guidebook uses built-in Rust implementations. Most common plugins (collapsible chapters, back-to-top, mermaid) are supported natively.

### Does it support PDF export?

Not currently. guidebook focuses on web output.

## Troubleshooting

### "Command not found: guidebook"

Add the install directory to your PATH:

```bash
export PATH="$PATH:$HOME/.local/bin"
```

Add this line to your `~/.zshrc` or `~/.bashrc`.

### Build fails with "SUMMARY.md not found"

Make sure you're running `guidebook build` in the directory containing `SUMMARY.md`.

### Images not showing

Check that image paths are relative to the markdown file:

```
![Image](./assets/image.png)
```
