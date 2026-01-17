use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd, CodeBlockKind, HeadingLevel};
use std::path::Path;

/// Table of Contents item
#[derive(Debug, Clone)]
pub struct TocItem {
    pub level: u8,
    pub text: String,
    pub id: String,
}

/// Extract headings from markdown content for TOC generation
pub fn extract_headings(content: &str) -> Vec<TocItem> {
    let content = fix_fullwidth_heading_spaces(content);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(&content, options);

    let mut headings = Vec::new();
    let mut in_heading: Option<HeadingLevel> = None;
    let mut heading_text = String::new();

    for event in parser {
        match &event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = Some(*level);
                heading_text.clear();
            }
            Event::Text(text) if in_heading.is_some() => {
                heading_text.push_str(text);
            }
            Event::End(TagEnd::Heading(level)) if in_heading.is_some() => {
                let level_num = heading_level_to_num(*level);
                // Only include h2, h3, h4 in TOC (skip h1 which is page title)
                if level_num >= 2 && level_num <= 4 {
                    let id = slugify(&heading_text);
                    headings.push(TocItem {
                        level: level_num,
                        text: heading_text.clone(),
                        id,
                    });
                }
                in_heading = None;
            }
            _ => {}
        }
    }

    headings
}

/// Render markdown content to HTML with Mermaid support
/// current_path: the path of the current markdown file (e.g., "Customer/AssetStatus/PortfolioTop.md")
/// hardbreaks: when true, treat single newlines as hard breaks (<br>)
pub fn render_markdown_with_path(content: &str, current_path: Option<&str>, hardbreaks: bool) -> String {
    // Normalize CRLF/CR to LF for consistent line handling
    let content = content.replace("\r\n", "\n").replace("\r", "\n");
    let html = render_markdown_internal(&content, hardbreaks);

    // If we have a current path, convert relative links to absolute
    if let Some(path) = current_path {
        convert_relative_links_to_absolute(&html, path)
    } else {
        html
    }
}

/// Render markdown content to HTML (backward compatible)
pub fn render_markdown(content: &str) -> String {
    // Normalize CRLF/CR to LF for consistent line handling
    let content = content.replace("\r\n", "\n").replace("\r", "\n");
    render_markdown_internal(&content, false)
}

/// Render markdown content to HTML with hardbreaks option
pub fn render_markdown_with_hardbreaks(content: &str, hardbreaks: bool) -> String {
    // Normalize CRLF/CR to LF for consistent line handling
    let content = content.replace("\r\n", "\n").replace("\r", "\n");
    render_markdown_internal(&content, hardbreaks)
}

fn render_markdown_internal(content: &str, hardbreaks: bool) -> String {
    // Strip all UTF-8 BOM characters (fixes reference link parsing issues)
    // BOM can appear at start of file or in concatenated content from @import
    let content = content.replace('\u{FEFF}', "");
    // Preprocess: fix full-width spaces after heading markers
    let content = fix_fullwidth_heading_spaces(&content);
    // Preprocess: fix image paths with spaces
    let content = fix_image_paths_with_spaces(&content);
    // Preprocess: fix multi-line footnotes without proper indentation
    let content = fix_multiline_footnotes(&content);
    // Preprocess: fix malformed table separator rows
    let content = fix_table_separator_columns(&content);

    // Convert footnote definitions to inline format (preserve original position)
    let content = convert_footnote_definitions_inline(&content, hardbreaks);

    // Convert footnote references [^n] to placeholders BEFORE markdown parsing
    // This prevents [A][^1] from being interpreted as a markdown link reference
    let content = convert_footnote_references_to_placeholder(&content);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    // Don't use pulldown-cmark's footnote processing - we handle it ourselves
    // options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(&content, options);

    // Process events to handle mermaid code blocks and heading IDs
    let mut in_mermaid = false;
    let mut mermaid_content = String::new();
    let mut in_heading: Option<HeadingLevel> = None;
    let mut heading_text = String::new();
    let mut custom_heading_id: Option<String> = None;  // Store custom ID from {#id} syntax
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
            // Track heading start and capture custom ID from {#id} syntax
            Event::Start(Tag::Heading { level, id, .. }) => {
                in_heading = Some(*level);
                heading_text.clear();
                // Capture custom ID if provided via {#custom-id} syntax
                custom_heading_id = id.as_ref().map(|s| s.to_string());
                events.push(event.clone());
                continue;
            }
            // Capture heading text
            Event::Text(text) if in_heading.is_some() => {
                heading_text.push_str(text);
                events.push(event.clone());
                continue;
            }
            // End of heading: inject ID
            Event::End(TagEnd::Heading(level)) if in_heading.is_some() => {
                // Use custom ID if provided, otherwise generate from heading text
                let id = custom_heading_id.take().unwrap_or_else(|| slugify(&heading_text));
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
            // Convert soft breaks to hard breaks when hardbreaks option is enabled
            Event::SoftBreak if hardbreaks => {
                events.push(Event::HardBreak);
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

    // Remove leading slashes from internal links
    html_output = remove_leading_slash_from_links(&html_output);

    // Auto-link URLs that are not already linked
    html_output = autolink_urls(&html_output);

    // Add target="_blank" to external links (Markdown-style links like [text](https://...))
    html_output = add_target_blank_to_external_links(&html_output);

    // Convert any remaining markdown images inside HTML blocks to <img> tags
    html_output = convert_remaining_markdown_images(&html_output);

    // Convert footnote placeholders to HTML
    html_output = convert_footnote_placeholders_to_html(&html_output);

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

/// Generate a URL-safe slug from text (matching github-slugger / HonKit behavior)
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else if c.is_whitespace() {
                Some('-')
            } else if c > '\x7F' {
                // Keep non-ASCII characters (Japanese, etc.)
                Some(c)
            } else {
                // Remove other special characters (/, ., etc.) to match github-slugger
                None
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

/// Collect reference link definitions from markdown content
/// Returns a map of label -> url
fn collect_reference_links(content: &str) -> std::collections::HashMap<String, String> {
    let mut links = std::collections::HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        // Match [label]: url pattern (but not footnote definitions [^n]:)
        if trimmed.starts_with('[') && !trimmed.starts_with("[^") {
            if let Some(bracket_end) = trimmed.find("]:") {
                let label = &trimmed[1..bracket_end];
                let url = trimmed[bracket_end + 2..].trim();
                if !label.is_empty() && !url.is_empty() {
                    // Remove optional angle brackets around URL
                    let url = url.trim_start_matches('<').trim_end_matches('>');
                    links.insert(label.to_lowercase(), url.to_string());
                }
            }
        }
    }

    links
}

/// Resolve reference links in text (e.g., [A] -> <a href="url">A</a>)
/// Handles both shortcut style [label] and full style [text][ref]
fn resolve_reference_links(text: &str, reference_links: &std::collections::HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut chars = text.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '[' {
            // Check for [label] or [text][ref] pattern
            let rest = &text[i + c.len_utf8()..];
            if let Some(end_byte) = rest.find(']') {
                let first_label = &rest[..end_byte];
                let after_bracket = &rest[end_byte + 1..];

                // Check for full reference link [text][ref]
                if after_bracket.starts_with('[') {
                    // Find the second closing bracket
                    let after_second_open = &after_bracket[1..];
                    if let Some(second_end_byte) = after_second_open.find(']') {
                        let ref_label = &after_second_open[..second_end_byte];
                        // Look up the reference (use ref_label, or first_label if ref is empty)
                        let lookup_key = if ref_label.is_empty() {
                            first_label.to_lowercase()
                        } else {
                            ref_label.to_lowercase()
                        };
                        if let Some(url) = reference_links.get(&lookup_key) {
                            result.push_str(&format!("<a href=\"{}\">{}</a>", url, first_label));
                            // Skip past [text][ref] - count characters (not bytes) to skip
                            // Pattern: [text][ref] - we need to skip: text + ] + [ + ref + ]
                            let chars_to_skip = first_label.chars().count() + 1 + 1 + ref_label.chars().count() + 1;
                            for _ in 0..chars_to_skip {
                                chars.next();
                            }
                            continue;
                        }
                    }
                }

                // Check for inline link [text](url) - skip these, pulldown-cmark handles them
                if after_bracket.starts_with('(') {
                    result.push(c);
                    continue;
                }

                // This is a shortcut reference link [label]
                if let Some(url) = reference_links.get(&first_label.to_lowercase()) {
                    result.push_str(&format!("<a href=\"{}\">{}</a>", url, first_label));
                    // Skip past the [label] - count characters to skip: label + ]
                    let chars_to_skip = first_label.chars().count() + 1;
                    for _ in 0..chars_to_skip {
                        chars.next();
                    }
                    continue;
                }
            }
        }
        result.push(c);
    }

    result
}

/// Convert footnote definitions in-place to HTML (preserve original position)
fn convert_footnote_definitions_inline(content: &str, hardbreaks: bool) -> String {
    // Collect reference link definitions for resolving within footnotes
    let reference_links = collect_reference_links(content);

    let mut result_lines = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        // Check if this line starts a footnote definition [^n]:
        if let Some(captures) = parse_footnote_def_start(line) {
            let (number, first_line_content) = captures;
            // Trim trailing whitespace from first line (for hardbreaks consistency)
            let first_line_content = first_line_content.trim_end();

            // Resolve reference links in the first line content
            let first_line_resolved = resolve_reference_links(first_line_content, &reference_links);

            let mut continuation_lines: Vec<String> = Vec::new();

            // Collect continuation lines (indented or list items until next footnote/heading/blank)
            i += 1;
            while i < lines.len() {
                let next_line = lines[i];
                let trimmed = next_line.trim_start();

                // Stop if: empty line, new footnote, heading
                if trimmed.is_empty() {
                    break;
                }
                if trimmed.starts_with("[^") && trimmed.contains("]:") {
                    break;
                }
                if trimmed.starts_with('#') {
                    break;
                }

                // This is a continuation line - resolve reference links
                let resolved_line = resolve_reference_links(next_line, &reference_links);
                continuation_lines.push(resolved_line);
                i += 1;
            }

            // Convert to inline HTML at original position (HonKit style)
            // First line goes inline with number, return link right after first line
            // Then continuation content (lists, etc.) follows
            let return_link = format!(
                "<a href=\"#reffn_{}\" title=\"Jump back to footnote [{}] in the text.\"> ↩</a>",
                number, number
            );

            if continuation_lines.is_empty() {
                // Single-line footnote: <blockquote><sup>n</sup>. content ↩</blockquote>
                // Use blockquote to match HonKit styling (left border)
                result_lines.push(format!(
                    "<blockquote id=\"fn_{}\"><sup>{}</sup>. {}{}</blockquote>",
                    number, number, first_line_resolved, return_link
                ));
            } else {
                // Multi-line footnote: first line in blockquote, continuation outside
                // This matches HonKit behavior: blockquote has border, continuation doesn't
                let continuation_content = continuation_lines.join("\n");
                let continuation_html = render_footnote_continuation(&continuation_content, hardbreaks);
                result_lines.push(format!(
                    "<blockquote id=\"fn_{}\"><sup>{}</sup>. {}{}</blockquote>\n{}",
                    number, number, first_line_resolved, return_link, continuation_html
                ));
            }
        } else {
            result_lines.push(line.to_string());
            i += 1;
        }
    }

    result_lines.join("\n")
}

/// Convert footnote references [^n] to placeholder (before parsing)
/// Placeholder format: %%FNREF_n%% - will be converted to HTML after markdown parsing
fn convert_footnote_references_to_placeholder(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '[' && content[i..].starts_with("[^") {
            // Find the closing ]
            let rest = &content[i + 2..];
            if let Some(end) = rest.find(']') {
                let number = &rest[..end];
                // Make sure it's a reference (not a definition - no : after ])
                let after = &rest[end + 1..];
                if !after.starts_with(':') && !number.is_empty() && number.chars().all(|c| c.is_alphanumeric()) {
                    // This is a reference, convert to placeholder
                    result.push_str(&format!("%%FNREF_{}%%", number));
                    // Skip past the reference: ^number]
                    // We already consumed '[', so skip: ^ + number + ]
                    for _ in 0..(1 + end + 1) {
                        chars.next();
                    }
                    continue;
                }
            }
        }
        result.push(c);
    }

    result
}

/// Convert footnote placeholders to HTML (after markdown parsing)
fn convert_footnote_placeholders_to_html(html: &str) -> String {
    let mut result = html.to_string();
    // Find all %%FNREF_n%% patterns and replace with HTML
    let re_pattern = "%%FNREF_";
    while let Some(start) = result.find(re_pattern) {
        let after_prefix = &result[start + re_pattern.len()..];
        if let Some(end) = after_prefix.find("%%") {
            let number = &after_prefix[..end];
            let replacement = format!(
                "<sup><a href=\"#fn_{}\" id=\"reffn_{}\">{}</a></sup>",
                number, number, number
            );
            let full_placeholder = format!("%%FNREF_{}%%", number);
            result = result.replacen(&full_placeholder, &replacement, 1);
        } else {
            break;
        }
    }
    result
}

/// Parse a footnote definition start line, returns (number, rest_of_line)
fn parse_footnote_def_start(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("[^") {
        return None;
    }

    // Find the closing ]
    let after_bracket = &trimmed[2..];
    let end_bracket = after_bracket.find("]:")?;
    let number = &after_bracket[..end_bracket];

    // Get the content after ]:
    let rest = &after_bracket[end_bracket + 2..].trim_start();
    Some((number, rest))
}


/// Render footnote continuation content (lists, paragraphs after first line)
fn render_footnote_continuation(content: &str, hardbreaks: bool) -> String {
    // Find minimum indentation (excluding empty lines) to preserve relative indentation
    let min_indent = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Remove only the common leading indentation, preserving relative structure
    let dedented: String = content
        .lines()
        .map(|line| {
            if line.len() >= min_indent {
                &line[min_indent..]
            } else {
                line.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(&dedented, options);

    // Apply hardbreaks conversion if enabled
    let events: Vec<Event> = parser.map(|event| {
        if hardbreaks {
            match event {
                Event::SoftBreak => Event::HardBreak,
                _ => event,
            }
        } else {
            event
        }
    }).collect();

    let mut html = String::new();
    html::push_html(&mut html, events.into_iter());

    html.trim().to_string()
}


/// Fix multi-line footnotes without proper indentation
/// Adds 4 spaces to ALL continuation lines to preserve relative indentation structure
fn fix_multiline_footnotes(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_footnote = false;

    for line in lines {
        // Check if this line starts a new footnote definition
        if line.starts_with("[^") && line.contains("]:") {
            in_footnote = true;
            result.push(line.to_string());
        } else if in_footnote {
            let trimmed = line.trim_start();

            if trimmed.is_empty() {
                // Empty line ends the footnote
                in_footnote = false;
                result.push(line.to_string());
            } else if trimmed.starts_with("[^") && trimmed.contains("]:") {
                // New footnote starts
                in_footnote = true;
                result.push(line.to_string());
            } else if trimmed.starts_with('#') {
                // Heading starts - end of footnotes section
                in_footnote = false;
                result.push(line.to_string());
            } else {
                // Continuation line - add 4 spaces to ALL lines to preserve relative structure
                result.push(format!("    {}", line));
            }
        } else {
            result.push(line.to_string());
        }
    }

    result.join("\n")
}

/// Fix malformed table rows:
/// - Add missing trailing | to header rows
/// - Fix separator rows where column count doesn't match header
fn fix_table_separator_columns(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Check if this line looks like a table row (starts with |)
        if trimmed.starts_with('|') {
            // Check if next line is a separator row
            if i + 1 < lines.len() {
                let next_line = lines[i + 1];
                if is_table_separator_row(next_line) {
                    // This is a header row - fix missing trailing pipe if needed
                    let fixed_header = fix_table_row_trailing_pipe(line);
                    let header_cols = count_table_columns(&fixed_header);

                    let separator_cols = count_table_columns(next_line);

                    // Push the fixed header
                    result.push(fixed_header);
                    i += 1;

                    // If column counts don't match, fix the separator row
                    if header_cols > 0 && separator_cols != header_cols {
                        let fixed_separator = generate_separator_row(header_cols, next_line);
                        result.push(fixed_separator);
                    } else {
                        result.push(next_line.to_string());
                    }
                    i += 1;
                    continue;
                }
            }
        }

        result.push(line.to_string());
        i += 1;
    }

    result.join("\n")
}

/// Add trailing pipe to table row if missing
fn fix_table_row_trailing_pipe(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.starts_with('|') && !trimmed.ends_with('|') {
        format!("{}|", line)
    } else {
        line.to_string()
    }
}

/// Count the number of columns in a table row
fn count_table_columns(line: &str) -> usize {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return 0;
    }

    // Count the | characters, accounting for leading/trailing pipes
    let pipe_count = trimmed.chars().filter(|&c| c == '|').count();

    // Number of columns = pipes - 1 (for |col1|col2|col3| format)
    if pipe_count > 1 {
        pipe_count - 1
    } else {
        0
    }
}

/// Check if a line is a table separator row (contains only |, -, :, and whitespace)
fn is_table_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return false;
    }

    // Must contain at least one dash
    if !trimmed.contains('-') {
        return false;
    }

    // All characters must be |, -, :, or whitespace
    trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c.is_whitespace())
}

/// Generate a separator row with the specified number of columns
/// Preserves per-column alignment from the original separator
fn generate_separator_row(col_count: usize, original: &str) -> String {
    // Parse alignments from original separator row
    let trimmed = original.trim();
    let original_alignments: Vec<&str> = trimmed
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|cell| {
            let cell = cell.trim();
            if cell.starts_with(':') && cell.ends_with(':') {
                ":--:"  // center
            } else if cell.starts_with(':') {
                ":--"   // left (explicit)
            } else if cell.ends_with(':') {
                "--:"   // right
            } else {
                "--"    // left (default)
            }
        })
        .collect();

    // Build new separator with correct number of columns
    // Use original alignments where available, default to "--" for extra columns
    let cols: Vec<&str> = (0..col_count)
        .map(|i| {
            if i < original_alignments.len() {
                original_alignments[i]
            } else {
                "--"  // default alignment for extra columns
            }
        })
        .collect();

    format!("|{}|", cols.join("|"))
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

    // Normalize backslashes to forward slashes in href attributes
    result = normalize_path_separators(&result);

    result
}

/// Remove leading slashes from internal links
/// Converts href="/path/to/file" → href="path/to/file"
/// Skips protocol-relative URLs (//example.com) and external links
fn remove_leading_slash_from_links(html: &str) -> String {
    let mut result = String::new();
    let mut chars = html.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        result.push(c);

        // Check for href=" or src="
        if c == '"' || c == '\'' {
            let quote_char = c;
            // Check if this is after href= or src= (check last 6 ASCII chars)
            let suffix: String = result.chars().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
            let is_href_or_src = suffix.to_lowercase().ends_with("href=") || suffix.to_lowercase().ends_with("src=");

            if is_href_or_src {
                // Collect the URL
                let mut url = String::new();
                while let Some((_, ch)) = chars.next() {
                    if ch == quote_char {
                        // Check if URL starts with single / (not //)
                        let processed_url = if url.starts_with('/') && !url.starts_with("//") {
                            // Check if it's an internal link (not external)
                            let lower = url.to_lowercase();
                            if !lower.starts_with("/http://") && !lower.starts_with("/https://") {
                                // Remove the leading slash
                                url.chars().skip(1).collect()
                            } else {
                                url
                            }
                        } else {
                            url
                        };
                        result.push_str(&processed_url);
                        result.push(quote_char);
                        break;
                    }
                    url.push(ch);
                }
            }
        }
    }

    result
}

/// Convert backslashes to forward slashes in href and src attributes
/// Handles Windows-style paths like href="path\to\file" → href="path/to/file"
fn normalize_path_separators(html: &str) -> String {
    let mut result = String::new();
    let mut chars = html.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        result.push(c);

        // Check for href=" or src="
        if c == '"' || c == '\'' {
            let quote_char = c;
            // Check if this is after href= or src= (check last 6 ASCII chars)
            let suffix: String = result.chars().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
            let is_href_or_src = suffix.to_lowercase().ends_with("href=") || suffix.to_lowercase().ends_with("src=");

            if is_href_or_src {
                // Collect the URL and normalize backslashes
                let mut url = String::new();
                while let Some((_, ch)) = chars.next() {
                    if ch == quote_char {
                        // Normalize backslashes to forward slashes
                        let normalized_url = url.replace('\\', "/");
                        result.push_str(&normalized_url);
                        result.push(quote_char);
                        break;
                    }
                    url.push(ch);
                }
            }
        }
    }

    result
}

/// Add target="_blank" rel="noopener noreferrer" to external links that don't have target attribute
/// This handles Markdown-style links [text](https://...) that were converted to <a href="...">
fn add_target_blank_to_external_links(html: &str) -> String {
    let mut result = String::new();
    let mut chars = html.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '<' && html[i..].starts_with("<a ") {
            // Found an anchor tag start
            let mut tag_content = String::from("<a ");
            // Skip past "<a "
            chars.next(); // 'a'
            chars.next(); // ' '

            // Collect the entire tag until '>'
            while let Some((_, ch)) = chars.next() {
                tag_content.push(ch);
                if ch == '>' {
                    break;
                }
            }

            // Check if this is an external link without target attribute
            let tag_lower = tag_content.to_lowercase();
            let has_target = tag_lower.contains("target=");
            let is_external = tag_lower.contains("href=\"http://") || tag_lower.contains("href=\"https://")
                || tag_lower.contains("href='http://") || tag_lower.contains("href='https://");

            if is_external && !has_target {
                // Insert target="_blank" rel="noopener noreferrer" before the closing >
                let without_close = tag_content.trim_end_matches('>');
                result.push_str(without_close);
                result.push_str(" target=\"_blank\" rel=\"noopener noreferrer\">");
            } else {
                result.push_str(&tag_content);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Auto-link URLs that are not already inside anchor tags or code blocks
/// Converts bare URLs like https://example.com to <a href="..." target="_blank">...</a>
fn autolink_urls(html: &str) -> String {
    let mut result = String::new();
    let mut chars = html.char_indices().peekable();
    let mut in_code = false;  // Track if we're inside <code> or <pre>

    while let Some((i, c)) = chars.next() {
        // Check if we're inside an HTML tag
        if c == '<' {
            result.push(c);

            // Collect the tag
            let mut tag_content = String::new();
            while let Some((_, ch)) = chars.next() {
                result.push(ch);
                if ch == '>' {
                    break;
                }
                tag_content.push(ch);
            }

            // Check for code/pre tags
            let tag_lower = tag_content.to_lowercase();
            if tag_lower.starts_with("code") || tag_lower.starts_with("pre") {
                in_code = true;
            } else if tag_lower.starts_with("/code") || tag_lower.starts_with("/pre") {
                in_code = false;
            }
            continue;
        }

        // Skip auto-linking if inside code block
        if in_code {
            result.push(c);
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

/// Convert remaining markdown image syntax ![alt](url) to <img> tags
/// This handles images inside raw HTML blocks that pulldown-cmark doesn't parse
/// Skips content inside <code> and <pre> tags
fn convert_remaining_markdown_images(html: &str) -> String {
    let mut result = String::new();
    let mut chars = html.char_indices().peekable();
    let mut in_code = false;  // Track if we're inside <code> or <pre>

    while let Some((_, c)) = chars.next() {
        // Check if we're inside an HTML tag
        if c == '<' {
            result.push(c);

            // Collect the tag
            let mut tag_content = String::new();
            while let Some((_, ch)) = chars.next() {
                result.push(ch);
                if ch == '>' {
                    break;
                }
                tag_content.push(ch);
            }

            // Check for code/pre tags
            let tag_lower = tag_content.to_lowercase();
            if tag_lower.starts_with("code") || tag_lower.starts_with("pre") {
                in_code = true;
            } else if tag_lower.starts_with("/code") || tag_lower.starts_with("/pre") {
                in_code = false;
            }
            continue;
        }

        // Skip image conversion if inside code block
        if in_code {
            result.push(c);
            continue;
        }

        if c == '!' && chars.peek().map(|(_, ch)| *ch) == Some('[') {
            chars.next(); // consume '['

            // Collect alt text until ']'
            let mut alt = String::new();
            let mut bracket_depth = 1;
            while let Some((_, ch)) = chars.next() {
                if ch == '[' {
                    bracket_depth += 1;
                    alt.push(ch);
                } else if ch == ']' {
                    bracket_depth -= 1;
                    if bracket_depth == 0 {
                        break;
                    }
                    alt.push(ch);
                } else {
                    alt.push(ch);
                }
            }

            // Check for '(' after ']'
            if chars.peek().map(|(_, ch)| *ch) == Some('(') {
                chars.next(); // consume '('

                // Collect URL until ')'
                let mut url = String::new();
                let mut paren_depth = 1;
                while let Some((_, ch)) = chars.next() {
                    if ch == '(' {
                        paren_depth += 1;
                        url.push(ch);
                    } else if ch == ')' {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            break;
                        }
                        url.push(ch);
                    } else {
                        url.push(ch);
                    }
                }

                // Output as <img> tag
                result.push_str(&format!(r#"<img src="{}" alt="{}">"#, url, alt));
            } else {
                // Not an image, output as-is
                result.push('!');
                result.push('[');
                result.push_str(&alt);
                result.push(']');
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert internal links to proper relative paths from current file
/// Links like "Customer/AssetStatus/PortfolioStock.html" (relative from book root)
/// need to be converted to "../../Customer/AssetStatus/PortfolioStock.html"
/// when rendered from a file at "Customer/AssetStatus/PortfolioTop.html"
/// current_path: e.g., "Customer/AssetStatus/PortfolioTop.md"
fn convert_relative_links_to_absolute(html: &str, current_path: &str) -> String {
    let result = html.to_string();

    // Calculate the depth (number of directories from root)
    // e.g., "Customer/AssetStatus/PortfolioTop.md" -> depth 2
    let depth = Path::new(current_path)
        .parent()
        .map(|p| {
            let dir = p.to_string_lossy();
            if dir.is_empty() {
                0
            } else {
                dir.matches('/').count() + 1
            }
        })
        .unwrap_or(0);

    // Create the prefix to go back to root (e.g., "../../" for depth 2)
    let root_prefix: String = "../".repeat(depth);

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

            // Check if this is an internal link that needs conversion
            // Skip: external links (http/https), anchor-only (#), already relative (../ or ./), absolute (/), data URIs
            let needs_conversion = !url.is_empty()
                && !url.starts_with("http://")
                && !url.starts_with("https://")
                && !url.starts_with('#')
                && !url.starts_with("../")
                && !url.starts_with("./")
                && !url.starts_with('/')
                && !url.starts_with("mailto:")
                && !url.starts_with("javascript:")
                && !url.starts_with("data:")
                && depth > 0;

            if needs_conversion {
                // Copy everything up to the URL
                new_result.push_str(&result[last_end..url_start]);
                // Add the root prefix + original URL
                new_result.push_str(&root_prefix);
                new_result.push_str(url);
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

// =============================================================================
// AsciiDoc Rendering
// =============================================================================

/// Render AsciiDoc content to HTML
/// Applies the same post-processing as markdown (target="_blank", link normalization, etc.)
pub fn render_asciidoc(content: &str) -> String {
    render_asciidoc_internal(content)
}

/// Render AsciiDoc content to HTML with path for relative link conversion
pub fn render_asciidoc_with_path(content: &str, current_path: Option<&str>) -> String {
    let html = render_asciidoc_internal(content);

    // If we have a current path, convert relative links to absolute
    if let Some(path) = current_path {
        convert_relative_links_to_absolute(&html, path)
    } else {
        html
    }
}

/// Extract headings from AsciiDoc content for TOC generation
pub fn extract_headings_from_asciidoc(content: &str) -> Vec<TocItem> {
    let mut headings = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // AsciiDoc headings: == Level 1, === Level 2, ==== Level 3, etc.
        if trimmed.starts_with("==") && !trimmed.starts_with("====") {
            // Count the equals signs
            let eq_count = trimmed.chars().take_while(|&c| c == '=').count();

            // Level mapping: == is h2, === is h3, ==== is h4
            // (= is h1 which is typically the document title)
            if eq_count >= 2 && eq_count <= 5 {
                let level = eq_count as u8;  // 2 = h2, 3 = h3, etc.
                let text = trimmed[eq_count..].trim().to_string();

                // Only include h2, h3, h4 in TOC (skip h1 which is page title)
                if level >= 2 && level <= 4 && !text.is_empty() {
                    let id = slugify(&text);
                    headings.push(TocItem {
                        level,
                        text,
                        id,
                    });
                }
            }
        }
    }

    headings
}

fn render_asciidoc_internal(content: &str) -> String {
    // Normalize CRLF/CR to LF for consistent line handling
    let content = content.replace("\r\n", "\n").replace("\r", "\n");

    // Strip all UTF-8 BOM characters
    let content = content.replace('\u{FEFF}', "");

    // Use asciidocr to convert to HTML
    // 1. Create a Scanner to tokenize the content
    let scanner = asciidocr::scanner::Scanner::new(&content);

    // 2. Create a Parser and parse the tokens (parser expects iterator of Result<Token, ScannerError>)
    let mut parser = asciidocr::parser::Parser::new(std::path::PathBuf::from("."));

    match parser.parse(scanner) {
        Ok(asg) => {
            // 3. Render the ASG to HTMLBook
            match asciidocr::backends::htmls::render_htmlbook(&asg) {
                Ok(html) => {
                    // Extract just the body content (asciidocr outputs full HTML document)
                    let html = extract_body_content(&html);

                    // Apply the same post-processing as markdown
                    let html = fix_asciidoc_relative_links(&html);
                    let html = remove_leading_slash_from_links(&html);
                    let html = autolink_urls(&html);
                    let html = add_target_blank_to_external_links(&html);

                    html
                }
                Err(e) => {
                    eprintln!("  Warning: AsciiDoc conversion error: {:?}", e);
                    format!("<p>{}</p>", html_escape(&content))
                }
            }
        }
        Err(e) => {
            eprintln!("  Warning: AsciiDoc parsing error: {:?}", e);
            // Return the content wrapped in a simple paragraph as fallback
            format!("<p>{}</p>", html_escape(&content))
        }
    }
}

/// Extract body content from full HTML document
/// asciidocr outputs full HTML with <!DOCTYPE>, <html>, <head>, <body>
/// We only need the content inside <body>
fn extract_body_content(html: &str) -> String {
    // Try to find <body> and </body> tags
    if let Some(body_start) = html.find("<body>") {
        let content_start = body_start + 6; // length of "<body>"
        if let Some(body_end) = html.find("</body>") {
            return html[content_start..body_end].trim().to_string();
        }
    }
    // If no body tags found, return as-is
    html.to_string()
}

/// Fix relative links in AsciiDoc output
/// Converts .adoc and .asciidoc links to .html
fn fix_asciidoc_relative_links(html: &str) -> String {
    let mut result = html.to_string();

    // Replace .adoc and .asciidoc links with .html
    let patterns = [
        (r#".adoc""#, r#".html""#),
        (r#".adoc#"#, r#".html#"#),
        (r#".adoc'"#, r#".html'"#),
        (r#".asciidoc""#, r#".html""#),
        (r#".asciidoc#"#, r#".html#"#),
        (r#".asciidoc'"#, r#".html'"#),
        // Also handle .md links for mixed content
        (r#".md""#, r#".html""#),
        (r#".md#"#, r#".html#"#),
        (r#".md'"#, r#".html'"#),
    ];

    for (from, to) in patterns {
        result = result.replace(from, to);
    }

    // Normalize backslashes to forward slashes in href attributes
    result = normalize_path_separators(&result);

    result
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

    #[test]
    fn test_multiline_footnotes() {
        let md = r#"Text with footnote[^1].

[^1]: First line
- Second line
- Third line
[^2]: Another footnote"#;
        let html = render_markdown(md);
        println!("Footnote HTML: {}", html);
        // The footnote should be properly rendered with list items inside
        assert!(html.contains("<li>"), "Footnote should contain list items: {}", html);
    }

    #[test]
    fn test_fix_multiline_footnotes_preprocessing() {
        let input = r#"[^1]: First line
- Second line
- Third line
[^2]: Another"#;
        let output = fix_multiline_footnotes(input);
        println!("Preprocessed:\n{}", output);
        assert!(output.contains("    - Second line"), "Second line should be indented: {}", output);
        assert!(output.contains("    - Third line"), "Third line should be indented: {}", output);
        assert!(!output.contains("    [^2]"), "New footnote should not be indented: {}", output);
    }

    #[test]
    fn test_slugify_matches_github_slugger() {
        // Test that slugify matches github-slugger / HonKit behavior
        // Special characters like / should be removed, not converted to hyphens
        assert_eq!(slugify("/auth/verification-email/resend"), "authverification-emailresend");
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("A.B.C"), "abc");  // Periods removed
        assert_eq!(slugify("日本語テスト"), "日本語テスト");  // Japanese preserved
        assert_eq!(slugify("test_underscore"), "test_underscore");  // Underscores preserved
        assert_eq!(slugify("a--b"), "a-b");  // Multiple hyphens collapsed
    }
}

#[test]
fn test_footnote_in_table() {
    let md = r#"| Col1 | Col2 | Col3 |
|------|------|------|
| [A][^1] | data | end |

[A]: #link
[^1]: Footnote one
"#;
    let html = render_markdown(md);
    println!("HTML: {}", html);
    // Check that table cells are separate
    assert!(html.contains("<td>data</td>") || html.contains(">data<"), "data should be in its own cell: {}", html);
}

#[test]
fn test_reference_link_basic() {
    // Test basic reference link functionality
    let md = r#"[改定履歴][AL_RH]

[AL_RH]: #改訂履歴"#;
    let html = render_markdown(md);
    println!("Test 1 (basic with space): {}", html);
    assert!(html.contains("<a "), "Reference link should create anchor: {}", html);
}

#[test]
fn test_reference_link_no_space() {
    // Test reference link without space after colon (HonKit format)
    let md = r#"[改定履歴][AL_RH]

[AL_RH]:#改訂履歴"#;
    let html = render_markdown(md);
    println!("Test 2 (no space): {}", html);
    // This might fail - checking pulldown-cmark behavior
    assert!(html.contains("<a "), "Reference link without space should work: {}", html);
}

#[test]
fn test_reference_link_after_html_comment() {
    // Test reference link definition after HTML comment
    let md = r#"[改定履歴][AL_RH]

<!-- 目次 -->
[AL_RH]: #改訂履歴"#;
    let html = render_markdown(md);
    println!("Test 3 (after HTML comment): {}", html);
    assert!(html.contains("<a "), "Reference link after HTML comment should work: {}", html);
}

#[test]
fn test_reference_link_after_html_comment_with_blank_line() {
    // Test reference link definition after HTML comment with blank line
    let md = r#"[改定履歴][AL_RH]

<!-- 目次 -->

[AL_RH]: #改訂履歴"#;
    let html = render_markdown(md);
    println!("Test 4 (after HTML comment with blank line): {}", html);
    assert!(html.contains("<a "), "Reference link after HTML comment with blank line should work: {}", html);
}

#[test]
fn test_reference_link_with_bom() {
    // Test reference link with UTF-8 BOM at start of definitions
    // BOM is \xEF\xBB\xBF (357 273 277 in octal)
    let bom = "\u{FEFF}";
    let md = format!(r#"[改定履歴][AL_RH]

{}<!-- 目次 -->
[AL_RH]:#改訂履歴"#, bom);
    let html = render_markdown(&md);
    println!("Test 5 (with BOM): {}", html);
    assert!(html.contains("<a "), "Reference link with BOM should work: {}", html);
}

#[test]
fn test_footnote_with_list() {
    let content = "- データソース項目の値\n- 上記以外の場合";
    let html = render_footnote_continuation(content, false);
    println!("Footnote continuation HTML: {}", html);
    assert!(html.contains("<li>") && html.contains("<ul>"), "Should contain list: {}", html);
}

#[test]
fn test_full_reference_link_in_footnote() {
    // Test [text][ref] pattern in footnotes - the bug that was reported
    let md = r#"Text[^1].

[^1]: .paymentAvailableStatus=[未申込][決済方法申込状態]の場合: "銀行引落(登録)"

[決済方法申込状態]:#決済方法申込状態"#;
    let html = render_markdown(md);
    println!("Full reference link in footnote: {}", html);
    // The [未申込] should become a link with href="#決済方法申込状態"
    assert!(html.contains("<a href=\"#決済方法申込状態\">未申込</a>"),
        "Full reference link [text][ref] should be resolved: {}", html);
    // The text after should be preserved
    assert!(html.contains("の場合:"),
        "Text after reference link should be preserved: {}", html);
}

#[test]
fn test_resolve_reference_links_full_style() {
    // Direct test of resolve_reference_links function with [text][ref] pattern
    let mut refs = std::collections::HashMap::new();
    refs.insert("決済方法申込状態".to_lowercase(), "#決済方法申込状態".to_string());

    let input = "[未申込][決済方法申込状態]の場合";
    let output = resolve_reference_links(input, &refs);
    println!("Resolved: {}", output);
    assert!(output.contains("<a href=\"#決済方法申込状態\">未申込</a>"),
        "Should resolve [text][ref]: {}", output);
    assert!(output.contains("の場合"),
        "Text after link should be preserved: {}", output);
}
