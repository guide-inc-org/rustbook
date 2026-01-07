mod renderer;
mod template;

use crate::parser::{self, apply_glossary, parse_front_matter, BookConfig, Glossary, Language, Summary, SummaryItem};
use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::Instant;

pub use renderer::{render_markdown, render_markdown_with_path, extract_headings, TocItem};
pub use template::Templates;

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
        // Expand variables before rendering
        let content = expand_variables(&parsed.content, config);
        let html_content = render_markdown(&content);
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
        fs::write(output.join("index.html"), page_html)?;
        stats.pages += 1;
    }

    // Generate search index (skip on hot reload for performance)
    if !skip_search_index {
        generate_search_index(source, output, &summary)?;
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
    let mut count = 0;

    for item in items {
        if let SummaryItem::Link { title, path, children } = item {
            if let Some(md_path) = path {
                let src_file = source.join(md_path);
                if src_file.exists() {
                    // Read and render markdown
                    let raw_content = fs::read_to_string(&src_file)?;
                    // Parse front matter
                    let parsed = parse_front_matter(&raw_content);
                    let front_matter = parsed.front_matter;
                    // Expand variables before rendering
                    let content = expand_variables(&parsed.content, config);
                    let html_content = render_markdown_with_path(&content, Some(md_path));
                    // Apply glossary terms
                    let html_content = apply_glossary(&html_content, glossary);
                    let toc_items = extract_headings(&content);

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

                    // Write output
                    if let Some(parent) = dest_file.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&dest_file, page_html)?;
                    count += 1;
                } else {
                    println!("  Warning: {} not found", md_path);
                }
            }

            // Build children recursively
            if !children.is_empty() {
                count += build_chapters(source, output, children, config, templates, summary, glossary)?;
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
            fs::copy(entry.path(), &dest_path)?;
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
            if let Some(md_path) = path {
                let src_file = source.join(md_path);
                if src_file.exists() {
                    let content = fs::read_to_string(&src_file)?;
                    let html_content = render_markdown(&content);
                    let text_content = strip_html_tags(&html_content);
                    let html_path = md_path.replace(".md", ".html");

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

/// Expand book variables in Markdown content
/// Replaces {{ book.xxx }} patterns with values from config.variables
fn expand_variables(content: &str, config: &BookConfig) -> String {
    if config.variables.is_empty() {
        return content.to_string();
    }

    // Match {{ book.xxx }} pattern with optional whitespace
    // Also match {{book.xxx}} without spaces
    let re = Regex::new(r"\{\{\s*book\.([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}").unwrap();

    re.replace_all(content, |caps: &regex::Captures| {
        let var_name = &caps[1];
        if let Some(value) = config.variables.get(var_name) {
            // Convert JSON value to string
            match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => value.to_string(), // For arrays/objects, use JSON representation
            }
        } else {
            // Variable not found, keep original text
            caps[0].to_string()
        }
    })
    .to_string()
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
}
