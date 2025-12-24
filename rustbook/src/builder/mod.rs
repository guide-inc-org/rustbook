mod renderer;
mod template;

use crate::parser::{self, BookConfig, Language, Summary, SummaryItem};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub use renderer::{render_markdown, render_markdown_with_path};
pub use template::Templates;

// Embed static assets at compile time
const GITBOOK_CSS: &str = include_str!("../../templates/gitbook.css");
const GITBOOK_JS: &str = include_str!("../../templates/gitbook.js");
const COLLAPSIBLE_JS: &str = include_str!("../../templates/collapsible.js");

/// Build the book from source directory to output directory
pub fn build(source: &Path, output: &Path) -> Result<()> {
    let source = source.canonicalize().context("Source directory not found")?;

    println!("Loading book configuration...");
    let config = BookConfig::load(&source)?;
    println!("  Title: {}", if config.title.is_empty() { "(untitled)" } else { &config.title });

    // Check for multi-language book
    let languages = parser::langs::parse_langs(&source)?;

    if languages.is_empty() {
        // Single language book
        println!("Building single-language book...");
        build_single_book(&source, output, &config)?;
    } else {
        // Multi-language book
        println!("Building multi-language book with {} languages:", languages.len());
        for lang in &languages {
            println!("  - {} ({})", lang.title, lang.code);
        }

        build_multi_lang_book(&source, output, &config, &languages)?;
    }

    println!("Build complete! Output at: {:?}", output);
    Ok(())
}

fn build_single_book(source: &Path, output: &Path, config: &BookConfig) -> Result<()> {
    let summary = Summary::parse(source)?;

    // Debug: print summary structure
    eprintln!("DEBUG: Summary has {} items", summary.items.len());
    for (i, item) in summary.items.iter().enumerate() {
        if let crate::parser::SummaryItem::Link { title, children, .. } = item {
            eprintln!("  DEBUG: Item {}: {} has {} children", i, title, children.len());
            for (j, child) in children.iter().enumerate() {
                if let crate::parser::SummaryItem::Link { title: ctitle, children: cchildren, .. } = child {
                    eprintln!("    DEBUG: Child {}: {} has {} grandchildren", j, ctitle, cchildren.len());
                }
            }
        }
    }

    let templates = Templates::new(config)?;

    // Create output directory
    fs::create_dir_all(output)?;

    // Write embedded static assets
    write_static_assets(output, config)?;

    // Copy assets
    copy_assets(source, output)?;

    // Copy custom styles if configured
    if let Some(style_path) = config.get_website_style() {
        let src_style = source.join(style_path);
        if src_style.exists() {
            let dest_style = output.join("gitbook/style.css");
            fs::create_dir_all(dest_style.parent().unwrap())?;
            fs::copy(&src_style, &dest_style)?;
        }
    }

    // Build each chapter
    build_chapters(source, output, &summary.items, config, &templates, &summary)?;

    // Generate index.html from README.md if exists
    let readme_path = source.join("README.md");
    if readme_path.exists() {
        let content = fs::read_to_string(&readme_path)?;
        let html_content = render_markdown(&content);
        let page_html = templates.render_page(
            &config.title,
            &html_content,
            "./",
            config,
            &summary,
            Some("index.html"),
        )?;
        fs::write(output.join("index.html"), page_html)?;
        println!("  Built: index.html");
    }

    Ok(())
}

fn write_static_assets(output: &Path, config: &BookConfig) -> Result<()> {
    let gitbook_dir = output.join("gitbook");
    fs::create_dir_all(&gitbook_dir)?;

    // Write CSS
    fs::write(gitbook_dir.join("gitbook.css"), GITBOOK_CSS)?;

    // Write JS
    fs::write(gitbook_dir.join("gitbook.js"), GITBOOK_JS)?;

    // Write collapsible JS if enabled
    if config.is_plugin_enabled("collapsible-chapters") {
        fs::write(gitbook_dir.join("collapsible.js"), COLLAPSIBLE_JS)?;
    }

    Ok(())
}

fn build_multi_lang_book(
    source: &Path,
    output: &Path,
    config: &BookConfig,
    languages: &[Language],
) -> Result<()> {
    // Create output directory
    fs::create_dir_all(output)?;

    // Generate language index page
    generate_lang_index(output, languages, config)?;

    // Build each language
    for lang in languages {
        println!("\nBuilding {} ({})...", lang.title, lang.code);
        let lang_source = source.join(&lang.code);
        let lang_output = output.join(&lang.code);

        // Use language-specific config if exists, otherwise use root config
        let lang_config_path = lang_source.join("book.json");
        let lang_config = if lang_config_path.exists() {
            BookConfig::load(&lang_source)?
        } else {
            config.clone()
        };

        build_single_book(&lang_source, &lang_output, &lang_config)?;
    }

    // Copy root assets if they exist
    let assets_dir = source.join("assets");
    if assets_dir.exists() {
        copy_dir_recursive(&assets_dir, &output.join("assets"))?;
    }

    Ok(())
}

fn build_chapters(
    source: &Path,
    output: &Path,
    items: &[SummaryItem],
    config: &BookConfig,
    templates: &Templates,
    summary: &Summary,
) -> Result<()> {
    for item in items {
        if let SummaryItem::Link { title, path, children } = item {
            if let Some(md_path) = path {
                let src_file = source.join(md_path);
                if src_file.exists() {
                    // Read and render markdown
                    let content = fs::read_to_string(&src_file)?;
                    let html_content = render_markdown_with_path(&content, Some(md_path));

                    // Generate output path
                    let html_path = md_path.replace(".md", ".html");
                    let dest_file = output.join(&html_path);

                    // Calculate relative path to root
                    let depth = html_path.matches('/').count();
                    let root_path = if depth > 0 {
                        "../".repeat(depth)
                    } else {
                        "./".to_string()
                    };

                    // Render with template
                    let page_html = templates.render_page(
                        title,
                        &html_content,
                        &root_path,
                        config,
                        summary,
                        Some(&html_path),
                    )?;

                    // Write output
                    if let Some(parent) = dest_file.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&dest_file, page_html)?;
                    println!("  Built: {}", html_path);
                } else {
                    println!("  Warning: {} not found", md_path);
                }
            }

            // Build children recursively
            if !children.is_empty() {
                build_chapters(source, output, children, config, templates, summary)?;
            }
        }
    }

    Ok(())
}

fn copy_assets(source: &Path, output: &Path) -> Result<()> {
    // Copy common asset directories
    for dir_name in &["assets", "images", "img"] {
        let src_dir = source.join(dir_name);
        if src_dir.exists() {
            let dest_dir = output.join(dir_name);
            copy_dir_recursive(&src_dir, &dest_dir)?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;

    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(src)?;
        let dest_path = dest.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &dest_path)?;
        }
    }

    Ok(())
}

fn generate_lang_index(output: &Path, languages: &[Language], config: &BookConfig) -> Result<()> {
    let title = if config.title.is_empty() {
        "Select Language"
    } else {
        &config.title
    };

    let mut lang_links = String::new();
    for lang in languages {
        lang_links.push_str(&format!(
            r#"<li><a href="{}/index.html">{}</a></li>"#,
            lang.code, lang.title
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{}</title>
    <style>
        body {{ font-family: sans-serif; max-width: 600px; margin: 50px auto; padding: 20px; }}
        h1 {{ color: #333; }}
        ul {{ list-style: none; padding: 0; }}
        li {{ margin: 10px 0; }}
        a {{ color: #4183c4; text-decoration: none; font-size: 1.2em; }}
        a:hover {{ text-decoration: underline; }}
    </style>
</head>
<body>
    <h1>{}</h1>
    <ul>{}</ul>
</body>
</html>"#,
        title, title, lang_links
    );

    fs::write(output.join("index.html"), html)?;
    Ok(())
}
