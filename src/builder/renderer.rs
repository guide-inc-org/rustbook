use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd, CodeBlockKind, HeadingLevel};
use std::path::Path;

/// Render markdown content to HTML with Mermaid support
/// current_path: the path of the current markdown file (e.g., "Customer/AssetStatus/PortfolioTop.md")
pub fn render_markdown_with_path(content: &str, current_path: Option<&str>) -> String {
    let html = render_markdown_internal(content);

    // If we have a current path, convert relative links to absolute
    if let Some(path) = current_path {
        convert_relative_links_to_absolute(&html, path)
    } else {
        html
    }
}

/// Render markdown content to HTML (backward compatible)
pub fn render_markdown(content: &str) -> String {
    render_markdown_internal(content)
}

fn render_markdown_internal(content: &str) -> String {
    // Preprocess: fix full-width spaces after heading markers
    let content = fix_fullwidth_heading_spaces(content);
    // Preprocess: fix image paths with spaces
    let content = fix_image_paths_with_spaces(&content);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(&content, options);

    // Process events to handle mermaid code blocks and heading IDs
    let mut in_mermaid = false;
    let mut mermaid_content = String::new();
    let mut in_heading: Option<HeadingLevel> = None;
    let mut heading_text = String::new();
    let mut events: Vec<Event> = Vec::new();

    for event in parser {
        match &event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                let lang_str = lang.as_ref();
                if lang_str == "mermaid" || lang_str.starts_with("mermaid") {
                    in_mermaid = true;
                    mermaid_content.clear();
                    continue;
                }
            }
            Event::End(TagEnd::CodeBlock) if in_mermaid => {
                // Output mermaid div instead of code block
                let mermaid_html = format!(
                    r#"<div class="mermaid">{}</div>"#,
                    html_escape(&mermaid_content)
                );
                events.push(Event::Html(mermaid_html.into()));
                in_mermaid = false;
                continue;
            }
            Event::Text(text) if in_mermaid => {
                mermaid_content.push_str(text);
                continue;
            }
            // Track heading start
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = Some(*level);
                heading_text.clear();
                events.push(event);
                continue;
            }
            // Capture heading text
            Event::Text(text) if in_heading.is_some() => {
                heading_text.push_str(text);
                events.push(event);
                continue;
            }
            // End of heading: inject ID
            Event::End(TagEnd::Heading(level)) if in_heading.is_some() => {
                let id = slugify(&heading_text);
                let level_num = heading_level_to_num(*level);
                // Pop the heading content and rebuild with ID
                let mut heading_events = Vec::new();
                while let Some(ev) = events.pop() {
                    if matches!(ev, Event::Start(Tag::Heading { .. })) {
                        break;
                    }
                    heading_events.push(ev);
                }
                heading_events.reverse();

                // Push heading with ID as raw HTML
                let open_tag = format!(r#"<h{} id="{}">"#, level_num, id);
                events.push(Event::Html(open_tag.into()));
                events.extend(heading_events);
                events.push(Event::Html(format!("</h{}>", level_num).into()));

                in_heading = None;
                continue;
            }
            _ => {}
        }
        events.push(event);
    }

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    // Fix relative links: convert .md to .html
    html_output = fix_relative_links(&html_output);

    // Auto-link URLs that are not already linked
    html_output = autolink_urls(&html_output);

    html_output
}

fn heading_level_to_num(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Generate a URL-safe slug from text
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else if c.is_whitespace() {
                Some('-')
            } else if c == '.' {
                // Remove periods (to match GitBook behavior)
                None
            } else if c > '\x7F' {
                // Keep non-ASCII characters (Japanese, etc.)
                Some(c)
            } else {
                Some('-')
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Fix full-width spaces after heading markers (common mistake in Japanese documents)
/// Converts "##　見出し" to "## 見出し"
fn fix_fullwidth_heading_spaces(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            // Check if line starts with heading markers followed by full-width space
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                // Find where the # sequence ends
                let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
                if hash_count > 0 && hash_count <= 6 {
                    let after_hashes = &trimmed[hash_count..];
                    // Check if followed by full-width space (U+3000)
                    if after_hashes.starts_with('\u{3000}') {
                        // Replace full-width space with half-width space
                        let leading_whitespace = &line[..line.len() - trimmed.len()];
                        let rest = &after_hashes['\u{3000}'.len_utf8()..];
                        return format!("{}{} {}", leading_whitespace, "#".repeat(hash_count), rest);
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Fix image paths that contain spaces by wrapping them in angle brackets
/// Converts ![alt](path with space.png) to ![alt](<path with space.png>)
fn fix_image_paths_with_spaces(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '!' {
            // Check for image syntax: ![...](...)
            if chars.peek() == Some(&'[') {
                // Collect the entire potential image syntax
                let mut img_str = String::from("!");
                img_str.push(chars.next().unwrap()); // '['

                // Read alt text until ']'
                let mut bracket_depth = 1;
                while let Some(&ch) = chars.peek() {
                    img_str.push(chars.next().unwrap());
                    if ch == '[' {
                        bracket_depth += 1;
                    } else if ch == ']' {
                        bracket_depth -= 1;
                        if bracket_depth == 0 {
                            break;
                        }
                    }
                }

                // Check for '(' after ']'
                if chars.peek() == Some(&'(') {
                    img_str.push(chars.next().unwrap()); // '('

                    // Read URL until ')'
                    let mut url = String::new();
                    let mut paren_depth = 1;
                    while let Some(&ch) = chars.peek() {
                        if ch == '(' {
                            paren_depth += 1;
                            url.push(chars.next().unwrap());
                        } else if ch == ')' {
                            paren_depth -= 1;
                            if paren_depth == 0 {
                                chars.next(); // consume ')'
                                break;
                            }
                            url.push(chars.next().unwrap());
                        } else {
                            url.push(chars.next().unwrap());
                        }
                    }

                    // Check if URL contains spaces and doesn't already use angle brackets
                    if url.contains(' ') && !url.starts_with('<') {
                        img_str.push('<');
                        img_str.push_str(&url);
                        img_str.push('>');
                    } else {
                        img_str.push_str(&url);
                    }
                    img_str.push(')');
                }

                result.push_str(&img_str);
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn fix_relative_links(html: &str) -> String {
    // Replace .md links with .html
    // Pattern: href="...*.md" or href='...*.md'
    let mut result = html.to_string();

    // Simple regex-like replacement for .md links
    // This handles href="path.md" and href="path.md#anchor"
    let patterns = [
        (r#".md""#, r#".html""#),
        (r#".md#"#, r#".html#"#),
        (r#".md'"#, r#".html'"#),
    ];

    for (from, to) in patterns {
        result = result.replace(from, to);
    }

    result
}

/// Auto-link URLs that are not already inside anchor tags
/// Converts bare URLs like https://example.com to <a href="..." target="_blank">...</a>
fn autolink_urls(html: &str) -> String {
    let mut result = String::new();
    let mut chars = html.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        // Check if we're inside an HTML tag
        if c == '<' {
            result.push(c);
            // Copy until we find '>'
            while let Some((_, ch)) = chars.next() {
                result.push(ch);
                if ch == '>' {
                    break;
                }
            }
            continue;
        }

        // Check for http:// or https://
        if c == 'h' && html[i..].starts_with("http://") || html[i..].starts_with("https://") {
            // Check if this URL is already inside an href=""
            if result.ends_with("href=\"") || result.ends_with("src=\"") {
                // Already in an href, just copy normally
                result.push(c);
                continue;
            }

            // Extract the URL
            let url_start = i;
            let mut url_end = i + 1;

            // Continue consuming URL characters
            while let Some(&(next_i, next_c)) = chars.peek() {
                // URL ends at whitespace, <, >, ", '
                if next_c.is_whitespace() || next_c == '<' || next_c == '>'
                    || next_c == '"' || next_c == '\'' {
                    break;
                }
                url_end = next_i + next_c.len_utf8();
                chars.next();
            }

            let mut url = &html[url_start..url_end];

            // Remove trailing punctuation that's likely not part of URL
            while url.ends_with('.') || url.ends_with(',') || url.ends_with(';')
                || url.ends_with(':') || url.ends_with(')') || url.ends_with('!') || url.ends_with('?') {
                url = &url[..url.len() - 1];
            }

            // Create the link with target="_blank"
            result.push_str(&format!(
                r#"<a href="{}" target="_blank">{}</a>"#,
                url, url
            ));

            // If we trimmed trailing punctuation, add it back
            let trimmed_len = url_end - url_start - url.len();
            if trimmed_len > 0 {
                result.push_str(&html[url_start + url.len()..url_end]);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert relative links (../ or ./) to absolute paths from root
/// Also convert anchor-only links (#id) to include current page path
/// current_path: e.g., "Customer/AssetStatus/PortfolioTop.md"
fn convert_relative_links_to_absolute(html: &str, current_path: &str) -> String {
    let result = html.to_string();

    // Get the directory of the current file
    let current_dir = Path::new(current_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Get the HTML path for the current file (for anchor links)
    let current_html_path = current_path.replace(".md", ".html");

    // Find and replace href="..." patterns
    let mut new_result = String::new();
    let mut last_end = 0;

    // Process href="..." patterns
    let href_pattern = r#"href=""#;
    let mut search_start = 0;

    while let Some(href_pos) = result[search_start..].find(href_pattern) {
        let abs_href_pos = search_start + href_pos;
        let url_start = abs_href_pos + href_pattern.len();

        // Find the closing quote
        if let Some(url_end_offset) = result[url_start..].find('"') {
            let url_end = url_start + url_end_offset;
            let url = &result[url_start..url_end];

            // Check if it's an anchor-only link (#id)
            if url.starts_with('#') {
                // Convert #anchor to current_page.html#anchor
                new_result.push_str(&result[last_end..url_start]);
                new_result.push_str(&current_html_path);
                new_result.push_str(url);
                last_end = url_end;
            }
            // Check if it's a relative path (starts with ../ or ./)
            else if url.starts_with("../") || url.starts_with("./") {
                // Copy everything up to the URL
                new_result.push_str(&result[last_end..url_start]);

                // Resolve the relative path
                let resolved = resolve_relative_path(&current_dir, url);
                new_result.push_str(&resolved);

                last_end = url_end;
            }

            search_start = url_end + 1;
        } else {
            search_start = url_start + 1;
        }
    }

    // Copy the remaining part
    new_result.push_str(&result[last_end..]);

    new_result
}

/// Resolve a relative path against a base directory
/// base_dir: e.g., "Customer/AssetStatus"
/// relative_path: e.g., "../Common/LocalStorage.html"
/// Returns: e.g., "Customer/Common/LocalStorage.html"
fn resolve_relative_path(base_dir: &str, relative_path: &str) -> String {
    let mut components: Vec<&str> = if base_dir.is_empty() {
        Vec::new()
    } else {
        base_dir.split('/').collect()
    };

    // Split path and anchor
    let (path_part, anchor) = if let Some(hash_pos) = relative_path.find('#') {
        (&relative_path[..hash_pos], &relative_path[hash_pos..])
    } else {
        (relative_path, "")
    };

    for part in path_part.split('/') {
        match part {
            ".." => {
                components.pop();
            }
            "." | "" => {}
            _ => {
                components.push(part);
            }
        }
    }

    format!("{}{}", components.join("/"), anchor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_basic_markdown() {
        let md = "# Hello\n\nThis is a **test**.";
        let html = render_markdown(md);
        // Heading now includes ID attribute
        assert!(html.contains("<h1 id=\"hello\">Hello</h1>"), "HTML: {}", html);
        assert!(html.contains("<strong>test</strong>"));
    }

    #[test]
    fn test_render_table() {
        let md = r#"
| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |
"#;
        let html = render_markdown(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Header 1</th>"));
    }

    #[test]
    fn test_render_mermaid() {
        let md = r#"
```mermaid
sequenceDiagram
    A->>B: Hello
```
"#;
        let html = render_markdown(md);
        assert!(html.contains(r#"<div class="mermaid">"#));
        assert!(html.contains("sequenceDiagram"));
    }

    #[test]
    fn test_fix_relative_links() {
        let html = r#"<a href="chapter1.md">Link</a>"#;
        let fixed = fix_relative_links(html);
        assert!(fixed.contains(r#"href="chapter1.html""#));
    }

    #[test]
    fn test_image_in_table() {
        let md = r#"
| Col1 | Col2 |
|:--:|:--:|
|![](test.png)|text|
"#;
        let html = render_markdown(md);
        println!("Generated HTML: {}", html);
        assert!(html.contains("<img"), "Image tag should be generated: {}", html);
    }

    #[test]
    fn test_image_in_table_japanese() {
        let md = r#"## デザイン
|該当するタイムラインがある場合|該当するタイムラインがない場合|
|:--:|:--:|
|![](../../../assets/Customer/TimeLine/B-0-8-Timeline Information Page.png)|![](../../../assets/Customer/TimeLine/B-0-8-Timeline Information Page0件.png)|
## 項目一覧"#;
        let html = render_markdown(md);
        println!("Generated HTML: {}", html);
        assert!(html.contains("<img"), "Image tag should be generated: {}", html);
    }

    #[test]
    fn test_image_with_space_in_filename() {
        // Test: space in filename
        let md = r#"|![](test file.png)|"#;
        let html = render_markdown(md);
        println!("With space: {}", html);

        // Test: no space in filename
        let md2 = r#"|![](testfile.png)|"#;
        let html2 = render_markdown(md2);
        println!("No space: {}", html2);
    }

    #[test]
    fn test_autolink_urls() {
        // Test: bare URL should become a link
        let md = "Guide Git:https://github.com/guide-inc-org/kcmsr-member-site-spec";
        let html = render_markdown(md);
        println!("Autolink result: {}", html);
        assert!(html.contains(r#"<a href="https://github.com/guide-inc-org/kcmsr-member-site-spec" target="_blank">"#),
            "URL should be auto-linked: {}", html);
    }

    #[test]
    fn test_autolink_does_not_double_link() {
        // Test: already linked URL should not be double-linked
        let md = "[Link](https://example.com)";
        let html = render_markdown(md);
        println!("Already linked result: {}", html);
        // Should have exactly one href for the URL
        let count = html.matches("https://example.com").count();
        assert_eq!(count, 1, "URL should appear only once: {}", html);
    }
}
