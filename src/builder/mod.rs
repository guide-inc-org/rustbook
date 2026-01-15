mod images;
mod nunjucks;
mod renderer;
pub mod svg;
mod template;

use crate::parser::{self, apply_glossary, parse_front_matter, BookConfig, Glossary, Language, Summary, SummaryItem};
use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

// nunjucks module is used internally for template processing
pub use renderer::{
    render_markdown, render_markdown_with_path, render_markdown_with_hardbreaks,
    render_asciidoc, render_asciidoc_with_path,
    extract_headings, extract_headings_from_asciidoc, TocItem
};
pub use template::Templates;

/// Check if a file is an AsciiDoc file based on its extension
pub fn is_asciidoc_file(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some("adoc") | Some("asciidoc") => true,
        _ => false,
    }
}

/// Search index entry
#[derive(Serialize)]
struct SearchEntry {
    title: String,
    path: String,
    content: String,
}

/// Build statistics
#[derive(Default)]
struct BuildStats {
    pages: usize,
    assets: usize,
}

// Embed static assets at compile time
const GITBOOK_CSS: &str = include_str!("../../templates/gitbook.css");
const GITBOOK_JS: &str = include_str!("../../templates/gitbook.js");
const COLLAPSIBLE_JS: &str = include_str!("../../templates/collapsible.js");
const FONTSETTINGS_JS: &str = include_str!("../../templates/fontsettings.js");
const SEARCH_JS: &str = include_str!("../../templates/search.js");

/// Build the book from source directory to output directory
pub fn build(source: &Path, output: &Path) -> Result<()> {
    build_with_options(source, output, false)
}

/// Build the book with options (skip_search_index for hot reload)
pub fn build_with_options(source: &Path, output: &Path, skip_search_index: bool) -> Result<()> {
    let start_time = Instant::now();
    let source = source.canonicalize().context("Source directory not found")?;

    println!("Loading book configuration...");
    let config = BookConfig::load(&source)?;
    println!("  Title: {}", if config.title.is_empty() { "(untitled)" } else { &config.title });

    // Check for multi-language book
    let languages = parser::langs::parse_langs(&source)?;

    let stats = if languages.is_empty() {
        // Single language book
        println!("Building single-language book...");
        build_single_book(&source, output, &config, skip_search_index)?
    } else {
        // Multi-language book
        println!("Building multi-language book with {} languages:", languages.len());
        for lang in &languages {
            println!("  - {} ({})", lang.title, lang.code);
        }

        build_multi_lang_book(&source, output, &config, &languages, skip_search_index)?
    };

    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();

    println!();
    println!(">> generation finished with success in {:.1}s !", elapsed_secs);
    println!("   {} pages built, {} asset files copied", stats.pages, stats.assets);

    Ok(())
}

fn build_single_book(source: &Path, output: &Path, config: &BookConfig, skip_search_index: bool) -> Result<BuildStats> {
    let summary = Summary::parse(source)?;
    let templates = Templates::new(config)?;
    let mut stats = BuildStats::default();

    // Load glossary if exists
    let glossary = Glossary::load(source)?;
    if !glossary.is_empty() {
        println!("  Loaded glossary with {} terms", glossary.entries.len());
    }

    // Create output directory
    fs::create_dir_all(output)?;

    // Write embedded static assets
    write_static_assets(output, config)?;

    // Copy assets
    stats.assets += copy_assets(source, output)?;

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
    stats.pages += build_chapters(source, output, &summary.items, config, &templates, &summary, &glossary)?;

    // Generate index.html from README.md if exists
    let readme_path = source.join("README.md");
    if readme_path.exists() {
        let raw_content = fs::read_to_string(&readme_path)?;
        // Parse front matter
        let parsed = parse_front_matter(&raw_content);
        let front_matter = parsed.front_matter;
        // Process @import directives before template processing
        let imported_content = process_imports_for_file(&parsed.content, &readme_path)?;
        // Process Nunjucks templates (conditionals, loops, filters, variables)
        let content = nunjucks::process_nunjucks_templates(&imported_content, config)
            .unwrap_or_else(|e| {
                eprintln!("  Warning: Template error in README.md: {}", e);
                imported_content.clone()
            });
        let html_content = render_markdown_with_hardbreaks(&content, config.hardbreaks);
        // Apply glossary terms
        let html_content = apply_glossary(&html_content, &glossary);
        let toc_items = extract_headings(&content);
        // Use front matter title if available, otherwise use config title
        let page_title = front_matter.as_ref()
            .and_then(|fm| fm.title.as_deref())
            .unwrap_or(&config.title);
        let page_html = templates.render_page_with_meta(
            page_title,
            &html_content,
            "./",
            config,
            &summary,
            Some("index.html"),
            &toc_items,
            front_matter.as_ref(),
        )?;
        // Apply SVG processing if configured
        let page_html = apply_svg_processing(page_html, output, config)?;
        fs::write(output.join("index.html"), page_html)?;
        stats.pages += 1;
    }

    // Generate search index (skip on hot reload for performance)
    if !skip_search_index {
        generate_search_index(source, output, &summary)?;
    }

    // Download remote images if enabled
    if config.fetch_remote_images {
        println!("Downloading remote images...");
        let downloaded = process_remote_images(output)?;
        if downloaded > 0 {
            println!("  Downloaded {} remote images", downloaded);
        }
    }

    Ok(stats)
}

fn write_static_assets(output: &Path, config: &BookConfig) -> Result<()> {
    let gitbook_dir = output.join("gitbook");
    fs::create_dir_all(&gitbook_dir)?;

    // Write CSS
    fs::write(gitbook_dir.join("gitbook.css"), GITBOOK_CSS)?;

    // Write JS
    fs::write(gitbook_dir.join("gitbook.js"), GITBOOK_JS)?;

    // Write collapsible JS only if plugin is enabled
    if config.is_plugin_enabled("collapsible-chapters") {
        fs::write(gitbook_dir.join("collapsible.js"), COLLAPSIBLE_JS)?;
    }

    // Write fontsettings JS only if plugin is enabled
    if config.is_plugin_enabled("fontsettings") {
        fs::write(gitbook_dir.join("fontsettings.js"), FONTSETTINGS_JS)?;
    }

    // Write search JS
    fs::write(gitbook_dir.join("search.js"), SEARCH_JS)?;

    Ok(())
}

fn build_multi_lang_book(
    source: &Path,
    output: &Path,
    config: &BookConfig,
    languages: &[Language],
    skip_search_index: bool,
) -> Result<BuildStats> {
    let mut stats = BuildStats::default();

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

        let lang_stats = build_single_book(&lang_source, &lang_output, &lang_config, skip_search_index)?;
        stats.pages += lang_stats.pages;
        stats.assets += lang_stats.assets;
    }

    // Copy root assets if they exist
    let assets_dir = source.join("assets");
    if assets_dir.exists() {
        stats.assets += copy_dir_recursive_count(&assets_dir, &output.join("assets"))?;
    }

    Ok(stats)
}

fn build_chapters(
    source: &Path,
    output: &Path,
    items: &[SummaryItem],
    config: &BookConfig,
    templates: &Templates,
    summary: &Summary,
    glossary: &Glossary,
) -> Result<usize> {
    let mut built_files: std::collections::HashSet<String> = std::collections::HashSet::new();
    build_chapters_inner(source, output, items, config, templates, summary, glossary, &mut built_files)
}

fn build_chapters_inner(
    source: &Path,
    output: &Path,
    items: &[SummaryItem],
    config: &BookConfig,
    templates: &Templates,
    summary: &Summary,
    glossary: &Glossary,
    built_files: &mut std::collections::HashSet<String>,
) -> Result<usize> {
    let mut count = 0;

    for item in items {
        if let SummaryItem::Link { title, path, children } = item {
            if let Some(md_path) = path {
                // Extract base file path (remove anchor #xxx if present)
                // Also strip leading slash to handle absolute-style paths in SUMMARY.md
                let base_path = if let Some(hash_pos) = md_path.find('#') {
                    md_path[..hash_pos].trim_start_matches('/')
                } else {
                    md_path.trim_start_matches('/')
                };

                // Skip if already built (avoid duplicate builds for anchor-only references)
                if base_path.is_empty() || built_files.contains(base_path) {
                    // Still need to process children
                    if !children.is_empty() {
                        count += build_chapters_inner(source, output, children, config, templates, summary, glossary, built_files)?;
                    }
                    continue;
                }

                let src_file = source.join(base_path);
                if src_file.exists() {
                    // Mark as built before processing
                    built_files.insert(base_path.to_string());

                    // Read file content
                    let raw_content = fs::read_to_string(&src_file)?;
                    // Parse front matter
                    let parsed = parse_front_matter(&raw_content);
                    let front_matter = parsed.front_matter;

                    // Check if this is an AsciiDoc file
                    let is_asciidoc = is_asciidoc_file(&src_file);

                    // Render content based on file type
                    let (html_content, toc_items) = if is_asciidoc {
                        // AsciiDoc rendering
                        let html = render_asciidoc_with_path(&parsed.content, Some(base_path));
                        let toc = extract_headings_from_asciidoc(&parsed.content);
                        (html, toc)
                    } else {
                        // Markdown rendering
                        // Process @import directives before template processing
                        let imported_content = process_imports_for_file(&parsed.content, &src_file)?;
                        // Process Nunjucks templates (conditionals, loops, filters, variables)
                        let content = nunjucks::process_nunjucks_templates(&imported_content, config)
                            .unwrap_or_else(|e| {
                                eprintln!("  Warning: Template error in {}: {}", base_path, e);
                                imported_content.clone()
                            });
                        let html = render_markdown_with_path(&content, Some(base_path), config.hardbreaks);
                        let toc = extract_headings(&content);
                        (html, toc)
                    };

                    // Apply glossary terms
                    let html_content = apply_glossary(&html_content, glossary);

                    // Generate output path (use base_path without anchor)
                    // Handle .md, .adoc, and .asciidoc extensions
                    let html_path = base_path
                        .replace(".md", ".html")
                        .replace(".adoc", ".html")
                        .replace(".asciidoc", ".html");
                    let dest_file = output.join(&html_path);

                    // Calculate relative path to root
                    let depth = html_path.matches('/').count();
                    let root_path = if depth > 0 {
                        "../".repeat(depth)
                    } else {
                        "./".to_string()
                    };

                    // Use front matter title if available, otherwise use summary title
                    let page_title = front_matter.as_ref()
                        .and_then(|fm| fm.title.as_deref())
                        .unwrap_or(title);

                    // Render with template
                    let page_html = templates.render_page_with_meta(
                        page_title,
                        &html_content,
                        &root_path,
                        config,
                        summary,
                        Some(&html_path),
                        &toc_items,
                        front_matter.as_ref(),
                    )?;

                    // Apply SVG processing if configured
                    let page_html = apply_svg_processing(page_html, output, config)?;

                    // Write output
                    if let Some(parent) = dest_file.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&dest_file, page_html)?;
                    count += 1;
                } else {
                    println!("  Warning: {} not found", base_path);
                }
            }

            // Build children recursively
            if !children.is_empty() {
                count += build_chapters_inner(source, output, children, config, templates, summary, glossary, built_files)?;
            }
        }
    }

    Ok(count)
}

fn copy_assets(source: &Path, output: &Path) -> Result<usize> {
    let mut count = 0;
    // Copy common asset directories
    for dir_name in &["assets", "images", "img"] {
        let src_dir = source.join(dir_name);
        if src_dir.exists() {
            let dest_dir = output.join(dir_name);
            count += copy_dir_recursive_count(&src_dir, &dest_dir)?;
        }
    }
    Ok(count)
}

fn copy_dir_recursive_count(src: &Path, dest: &Path) -> Result<usize> {
    fs::create_dir_all(dest)?;
    let mut count = 0;

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
            // Use symlinks on Unix for faster builds (no actual file copy)
            // Falls back to copy on Windows
            #[cfg(unix)]
            {
                let abs_src = entry.path().canonicalize()?;
                std::os::unix::fs::symlink(&abs_src, &dest_path)?;
            }
            #[cfg(not(unix))]
            {
                fs::copy(entry.path(), &dest_path)?;
            }
            count += 1;
        }
    }

    Ok(count)
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
            r#"
            <li>
                <a href="{}/">{}</a>
            </li>
        "#,
            lang.code, lang.title
        ));
    }

    let html = format!(
        r#"<!DOCTYPE HTML>
<html lang="" >
    <head>
        <meta charset="UTF-8">
        <title>Choose a language · {}</title>
        <meta http-equiv="X-UA-Compatible" content="IE=edge" />
        <meta name="description" content="">
        <meta name="generator" content="guidebook">
        <link rel="stylesheet" href="gitbook/style.css">
        <meta name="HandheldFriendly" content="true"/>
        <meta name="viewport" content="width=device-width, initial-scale=1, user-scalable=no">
        <meta name="apple-mobile-web-app-capable" content="yes">
        <meta name="apple-mobile-web-app-status-bar-style" content="black">
        <link rel="apple-touch-icon-precomposed" sizes="152x152" href="gitbook/images/apple-touch-icon-precomposed-152.png">
        <link rel="shortcut icon" href="gitbook/images/favicon.ico" type="image/x-icon">
    </head>
    <body>

<div class="book-langs-index" role="navigation">
    <div class="inner">
        <h3>Choose a language</h3>

        <ul class="languages">
        {}
        </ul>
    </div>
</div>

    </body>
</html>"#,
        title, lang_links
    );

    fs::write(output.join("index.html"), html)?;

    // Copy gitbook static files to root for the language selector page
    copy_gitbook_static_to_root(output)?;

    Ok(())
}

/// Strip HTML tags from content for search indexing
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }

    // Clean up whitespace
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Collect search entries from summary items
fn collect_search_entries(
    source: &Path,
    items: &[SummaryItem],
    entries: &mut Vec<SearchEntry>,
) -> Result<()> {
    for item in items {
        if let SummaryItem::Link { title, path, children } = item {
            if let Some(file_path) = path {
                // Strip leading slash to handle absolute-style paths in SUMMARY.md
                let file_path = file_path.trim_start_matches('/');
                let src_file = source.join(file_path);
                if src_file.exists() {
                    let content = fs::read_to_string(&src_file)?;

                    // Render based on file type
                    let html_content = if is_asciidoc_file(&src_file) {
                        render_asciidoc(&content)
                    } else {
                        render_markdown(&content)
                    };

                    let text_content = strip_html_tags(&html_content);

                    // Generate HTML path for any supported extension
                    let html_path = file_path
                        .replace(".md", ".html")
                        .replace(".adoc", ".html")
                        .replace(".asciidoc", ".html");

                    entries.push(SearchEntry {
                        title: title.clone(),
                        path: html_path,
                        content: text_content,
                    });
                }
            }
            if !children.is_empty() {
                collect_search_entries(source, children, entries)?;
            }
        }
    }
    Ok(())
}

/// Generate search index JSON file
fn generate_search_index(source: &Path, output: &Path, summary: &Summary) -> Result<()> {
    let mut entries = Vec::new();

    // Collect from README.md
    let readme_path = source.join("README.md");
    if readme_path.exists() {
        let content = fs::read_to_string(&readme_path)?;
        let html_content = render_markdown(&content);
        let text_content = strip_html_tags(&html_content);

        entries.push(SearchEntry {
            title: "Home".to_string(),
            path: "index.html".to_string(),
            content: text_content,
        });
    }

    // Collect from all chapters
    collect_search_entries(source, &summary.items, &mut entries)?;

    // Write search index
    let json = serde_json::to_string(&entries)?;
    fs::write(output.join("search_index.json"), json)?;

    Ok(())
}

/// Process all HTML files in output directory to download remote images
/// Returns the number of images downloaded
fn process_remote_images(output: &Path) -> Result<usize> {
    use images::ImageDownloader;

    let mut downloader = ImageDownloader::new(output);

    // Walk through all HTML files in output directory
    for entry in walkdir::WalkDir::new(output) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext == "html" {
                    // Read HTML file
                    let html = fs::read_to_string(entry.path())?;

                    // Process remote images
                    match downloader.process_html(&html) {
                        Ok(processed_html) => {
                            // Only write back if content changed
                            if processed_html != html {
                                fs::write(entry.path(), processed_html)?;
                            }
                        }
                        Err(e) => {
                            eprintln!("  Warning: Failed to process {}: {}", entry.path().display(), e);
                        }
                    }
                }
            }
        }
    }

    let (downloaded, _) = downloader.stats();
    Ok(downloaded)
}

/// Process @import directives in Markdown content
/// Replaces <!-- @import("path/to/file.md") --> with the contents of the referenced file
/// Supports recursive imports with loop prevention
fn process_imports(content: &str, base_path: &Path, visited: &mut HashSet<PathBuf>) -> Result<String> {
    // Regex to match <!-- @import("path/to/file") --> with optional whitespace
    let re = Regex::new(r#"<!--\s*@import\s*\(\s*"([^"]+)"\s*\)\s*-->"#).unwrap();

    let mut result = content.to_string();
    let mut offset: i64 = 0;

    for caps in re.captures_iter(content) {
        let full_match = caps.get(0).unwrap();
        let import_path = &caps[1];

        // Resolve the path relative to the base_path (directory containing the current file)
        let resolved_path = base_path.join(import_path);
        let canonical_path = match resolved_path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // File doesn't exist, leave the directive as-is and warn
                eprintln!("  Warning: @import file not found: {}", resolved_path.display());
                continue;
            }
        };

        // Check for circular imports
        if visited.contains(&canonical_path) {
            eprintln!("  Warning: Circular @import detected, skipping: {}", canonical_path.display());
            continue;
        }

        // Mark this file as visited
        visited.insert(canonical_path.clone());

        // Read the imported file
        let imported_content = match fs::read_to_string(&canonical_path) {
            Ok(c) => {
                // Strip UTF-8 BOM if present (fixes reference link parsing)
                c.strip_prefix('\u{FEFF}').unwrap_or(&c).to_string()
            },
            Err(e) => {
                eprintln!("  Warning: Failed to read @import file {}: {}", canonical_path.display(), e);
                continue;
            }
        };

        // Recursively process imports in the imported content
        // Use the directory of the imported file as the new base path
        let import_base_path = canonical_path.parent().unwrap_or(base_path);
        let processed_content = process_imports(&imported_content, import_base_path, visited)?;

        // Calculate the adjusted positions accounting for previous replacements
        let start = (full_match.start() as i64 + offset) as usize;
        let end = (full_match.end() as i64 + offset) as usize;

        // Replace the directive with the processed content
        result.replace_range(start..end, &processed_content);

        // Update offset for subsequent replacements
        offset += processed_content.len() as i64 - (full_match.end() - full_match.start()) as i64;
    }

    Ok(result)
}

/// Process @import directives starting from a file path
/// This is a convenience wrapper that initializes the visited set
fn process_imports_for_file(content: &str, file_path: &Path) -> Result<String> {
    let mut visited = HashSet::new();

    // Add the current file to visited set to prevent self-imports
    if let Ok(canonical) = file_path.canonicalize() {
        visited.insert(canonical);
    }

    // Get the directory containing the file as the base path
    let base_path = file_path.parent().unwrap_or(Path::new("."));

    process_imports(content, base_path, &mut visited)
}

/// Apply SVG processing to HTML based on config options
fn apply_svg_processing(html: String, output_dir: &Path, config: &BookConfig) -> Result<String> {
    let mut result = html;

    // Apply externalize_svg if enabled
    if config.externalize_svg == Some(true) {
        result = svg::externalize_inline_svg(&result, output_dir)?;
    }

    // Apply inline_svg if enabled
    if config.inline_svg == Some(true) {
        result = svg::inline_svg_files(&result, output_dir)?;
    }

    Ok(result)
}

/// Expand book variables in Markdown content (legacy implementation)
/// Note: This is now superseded by nunjucks::process_nunjucks_templates
/// but kept for backward compatibility tests
/// Replaces {{ book.xxx }} patterns with values from config.variables
/// Preserves variables inside code blocks (``` ... ```) and inline code (` ... `)
#[cfg(test)]
fn expand_variables(content: &str, config: &BookConfig) -> String {
    if config.variables.is_empty() {
        return content.to_string();
    }

    // Strategy: Find protected regions (code blocks and inline code) first,
    // then only apply variable expansion outside these regions

    // Find all protected regions (fenced code blocks and inline code)
    let protected_regions = find_protected_regions(content);

    // Match {{ book.xxx }} pattern with optional whitespace
    let var_re = Regex::new(r"\{\{\s*book\.([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}").unwrap();

    let mut result = String::new();
    let mut last_end = 0;

    for caps in var_re.captures_iter(content) {
        let full_match = caps.get(0).unwrap();
        let start = full_match.start();
        let end = full_match.end();

        // Check if this match is inside a protected region
        let is_protected = protected_regions.iter().any(|(region_start, region_end)| {
            start >= *region_start && end <= *region_end
        });

        // Add content before this match
        result.push_str(&content[last_end..start]);

        if is_protected {
            // Inside code block/inline code - keep original
            result.push_str(&content[start..end]);
        } else {
            // Outside code - perform replacement
            let var_name = &caps[1];
            if let Some(value) = config.variables.get(var_name) {
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                result.push_str(&replacement);
            } else {
                // Variable not found, keep original text
                result.push_str(&content[start..end]);
            }
        }

        last_end = end;
    }

    // Add remaining content after last match
    result.push_str(&content[last_end..]);

    result
}

/// Find all protected regions in the content (code blocks and inline code)
/// Returns a vector of (start, end) byte positions
/// Note: This is now superseded by nunjucks module's protected region handling
/// but kept for backward compatibility tests
#[cfg(test)]
fn find_protected_regions(content: &str) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();

    // Find fenced code blocks (``` ... ```) - must come first as they take priority
    let fenced_re = Regex::new(r"(?s)```[^\n]*\n.*?```").unwrap();
    for m in fenced_re.find_iter(content) {
        regions.push((m.start(), m.end()));
    }

    // Find inline code (` ... `) but not if inside fenced blocks
    let inline_re = Regex::new(r"`[^`\n]+`").unwrap();
    for m in inline_re.find_iter(content) {
        // Only add if not overlapping with existing regions
        let overlaps = regions.iter().any(|(start, end)| {
            m.start() >= *start && m.end() <= *end
        });
        if !overlaps {
            regions.push((m.start(), m.end()));
        }
    }

    regions
}

fn copy_gitbook_static_to_root(output: &Path) -> Result<()> {
    let gitbook_dir = output.join("gitbook");
    fs::create_dir_all(&gitbook_dir)?;

    // Create a minimal style.css for the language selector page
    let style_css = r#"
.book-langs-index {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 100vh;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
}

.book-langs-index .inner {
    text-align: center;
}

.book-langs-index h3 {
    color: #333;
    font-size: 1.5em;
    margin-bottom: 1em;
}

.book-langs-index .languages {
    list-style: none;
    padding: 0;
    margin: 0;
}

.book-langs-index .languages li {
    margin: 0.5em 0;
}

.book-langs-index .languages a {
    color: #4183c4;
    text-decoration: none;
    font-size: 1.2em;
}

.book-langs-index .languages a:hover {
    text-decoration: underline;
}
"#;

    fs::write(gitbook_dir.join("style.css"), style_css)?;

    // Create images directory with placeholder favicon
    let images_dir = gitbook_dir.join("images");
    fs::create_dir_all(&images_dir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config(variables: HashMap<String, serde_json::Value>) -> BookConfig {
        BookConfig {
            variables,
            ..Default::default()
        }
    }

    #[test]
    fn test_expand_variables_basic() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("1.0.0"));
        vars.insert("author".to_string(), serde_json::json!("Guide Inc"));

        let config = create_test_config(vars);
        let content = "Version: {{ book.version }}\nAuthor: {{ book.author }}";
        let result = expand_variables(content, &config);

        assert_eq!(result, "Version: 1.0.0\nAuthor: Guide Inc");
    }

    #[test]
    fn test_expand_variables_no_spaces() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("2.0.0"));

        let config = create_test_config(vars);
        let content = "Version: {{book.version}}";
        let result = expand_variables(content, &config);

        assert_eq!(result, "Version: 2.0.0");
    }

    #[test]
    fn test_expand_variables_with_extra_spaces() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("Test"));

        let config = create_test_config(vars);
        let content = "Name: {{  book.name  }}";
        let result = expand_variables(content, &config);

        assert_eq!(result, "Name: Test");
    }

    #[test]
    fn test_expand_variables_number() {
        let mut vars = HashMap::new();
        vars.insert("year".to_string(), serde_json::json!(2024));

        let config = create_test_config(vars);
        let content = "Year: {{ book.year }}";
        let result = expand_variables(content, &config);

        assert_eq!(result, "Year: 2024");
    }

    #[test]
    fn test_expand_variables_boolean() {
        let mut vars = HashMap::new();
        vars.insert("published".to_string(), serde_json::json!(true));

        let config = create_test_config(vars);
        let content = "Published: {{ book.published }}";
        let result = expand_variables(content, &config);

        assert_eq!(result, "Published: true");
    }

    #[test]
    fn test_expand_variables_unknown_variable() {
        let mut vars = HashMap::new();
        vars.insert("known".to_string(), serde_json::json!("value"));

        let config = create_test_config(vars);
        let content = "Known: {{ book.known }}, Unknown: {{ book.unknown }}";
        let result = expand_variables(content, &config);

        // Unknown variable should remain unchanged
        assert_eq!(result, "Known: value, Unknown: {{ book.unknown }}");
    }

    #[test]
    fn test_expand_variables_empty_config() {
        let config = create_test_config(HashMap::new());
        let content = "No variables: {{ book.test }}";
        let result = expand_variables(content, &config);

        // Should return content unchanged
        assert_eq!(result, "No variables: {{ book.test }}");
    }

    #[test]
    fn test_expand_variables_in_markdown() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("1.0.0"));

        let config = create_test_config(vars);
        let content = "# Version {{ book.version }}\n\nThis is version {{ book.version }}.";
        let result = expand_variables(content, &config);

        assert_eq!(result, "# Version 1.0.0\n\nThis is version 1.0.0.");
    }

    #[test]
    fn test_expand_variables_preserves_code_blocks() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("1.0.0"));

        let config = create_test_config(vars);
        let content = r#"Version: {{ book.version }}

```javascript
// This should not be expanded
const version = "{{ book.version }}";
console.log(version);
```

After code block: {{ book.version }}"#;

        let result = expand_variables(content, &config);

        // Variables outside code blocks should be expanded
        assert!(result.contains("Version: 1.0.0"));
        assert!(result.contains("After code block: 1.0.0"));
        // Variables inside code blocks should NOT be expanded
        assert!(result.contains(r#"const version = "{{ book.version }}";"#));
    }

    #[test]
    fn test_expand_variables_preserves_inline_code() {
        let mut vars = HashMap::new();
        vars.insert("var".to_string(), serde_json::json!("value"));

        let config = create_test_config(vars);
        let content = "Normal: {{ book.var }}, inline: `{{ book.var }}`, after: {{ book.var }}";
        let result = expand_variables(content, &config);

        assert_eq!(result, "Normal: value, inline: `{{ book.var }}`, after: value");
    }

    #[test]
    fn test_expand_variables_multiple_code_blocks() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), serde_json::json!("X"));

        let config = create_test_config(vars);
        let content = r#"{{ book.x }}
```
{{ book.x }}
```
{{ book.x }}
```rust
{{ book.x }}
```
{{ book.x }}"#;

        let result = expand_variables(content, &config);

        // Count occurrences of "X" (expanded) and "{{ book.x }}" (not expanded)
        let x_count = result.matches("X").count();
        let template_count = result.matches("{{ book.x }}").count();

        // 3 outside code blocks should be expanded
        assert_eq!(x_count, 3);
        // 2 inside code blocks should NOT be expanded
        assert_eq!(template_count, 2);
    }

    #[test]
    fn test_find_protected_regions_fenced_code() {
        let content = "text\n```\ncode\n```\nmore text";
        let regions = find_protected_regions(content);

        assert_eq!(regions.len(), 1);
        // The region should cover the entire code block
        let (start, end) = regions[0];
        assert!(content[start..end].starts_with("```"));
        assert!(content[start..end].ends_with("```"));
    }

    #[test]
    fn test_find_protected_regions_inline_code() {
        let content = "text `inline` more text";
        let regions = find_protected_regions(content);

        assert_eq!(regions.len(), 1);
        let (start, end) = regions[0];
        assert_eq!(&content[start..end], "`inline`");
    }

    #[test]
    fn test_find_protected_regions_multiple() {
        let content = "`a` text `b` more\n```\nblock\n```\nend";
        let regions = find_protected_regions(content);

        // Should find: 1 fenced block + 2 inline codes
        assert_eq!(regions.len(), 3);
    }

    #[test]
    fn test_process_imports_regex_pattern() {
        // Test the regex pattern matches correctly
        let re = Regex::new(r#"<!--\s*@import\s*\(\s*"([^"]+)"\s*\)\s*-->"#).unwrap();

        // Should match
        assert!(re.is_match(r#"<!-- @import("file.md") -->"#));
        assert!(re.is_match(r#"<!--@import("file.md")-->"#));
        assert!(re.is_match(r#"<!--  @import( "file.md" )  -->"#));
        assert!(re.is_match(r#"<!-- @import("path/to/file.md") -->"#));

        // Should not match
        assert!(!re.is_match(r#"@import("file.md")"#)); // No HTML comment
        assert!(!re.is_match(r#"<!-- @import('file.md') -->"#)); // Single quotes
    }
}
