use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Glossary containing all terms and their definitions
#[derive(Debug, Clone, Default)]
pub struct Glossary {
    /// Map from term to definition
    pub entries: HashMap<String, String>,
    /// Terms sorted by length (longest first) for replacement
    pub sorted_terms: Vec<String>,
}

impl Glossary {
    /// Load glossary from GLOSSARY.md file
    pub fn load(book_dir: &Path) -> Result<Self> {
        let glossary_path = book_dir.join("GLOSSARY.md");
        if !glossary_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&glossary_path)?;
        Self::parse(&content)
    }

    /// Parse GLOSSARY.md content
    pub fn parse(content: &str) -> Result<Self> {
        let mut entries = HashMap::new();
        let mut current_term: Option<String> = None;
        let mut current_definition = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip the main heading (# GLOSSARY)
            if trimmed.starts_with("# ") {
                continue;
            }

            // Check for term heading (## Term)
            if trimmed.starts_with("## ") {
                // Save previous entry if exists
                if let Some(term) = current_term.take() {
                    let definition = current_definition.trim().to_string();
                    if !definition.is_empty() {
                        entries.insert(term, definition);
                    }
                }

                // Start new entry
                current_term = Some(trimmed[3..].trim().to_string());
                current_definition.clear();
                continue;
            }

            // Accumulate definition lines
            if current_term.is_some() && !trimmed.is_empty() {
                if !current_definition.is_empty() {
                    current_definition.push(' ');
                }
                current_definition.push_str(trimmed);
            }
        }

        // Save last entry
        if let Some(term) = current_term {
            let definition = current_definition.trim().to_string();
            if !definition.is_empty() {
                entries.insert(term, definition);
            }
        }

        // Sort terms by length (longest first) to avoid partial replacements
        let mut sorted_terms: Vec<String> = entries.keys().cloned().collect();
        sorted_terms.sort_by(|a, b| b.len().cmp(&a.len()));

        Ok(Self {
            entries,
            sorted_terms,
        })
    }

    /// Check if glossary is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get definition for a term
    pub fn get(&self, term: &str) -> Option<&String> {
        self.entries.get(term)
    }
}

/// Apply glossary terms to HTML content
/// Wraps matching terms in <span class="glossary-term" data-definition="...">
pub fn apply_glossary(html: &str, glossary: &Glossary) -> String {
    if glossary.is_empty() {
        return html.to_string();
    }

    let mut result = html.to_string();

    // Process each term (longest first to avoid partial replacements)
    for term in &glossary.sorted_terms {
        if let Some(definition) = glossary.get(term) {
            result = replace_term_in_html(&result, term, definition);
        }
    }

    result
}

/// Replace a term in HTML content, avoiding replacements inside:
/// - HTML tags
/// - Existing glossary spans
/// - Code blocks (<code>, <pre>)
/// - Anchor tags (<a>)
/// - Heading tags (<h1> through <h6>)
/// - Script tags (<script>)
/// - Elements with class="no-glossary"
/// - Already processed terms
fn replace_term_in_html(html: &str, term: &str, definition: &str) -> String {
    let mut result = String::new();
    let mut chars = html.char_indices().peekable();
    let mut in_tag = false;
    let mut in_code = false;
    let mut in_glossary_span = false;
    let mut in_anchor = false;
    let mut in_heading = false;
    let mut in_script = false;
    let mut no_glossary_stack: Vec<String> = Vec::new(); // Stack of tag names with no-glossary class
    let mut tag_content = String::new();

    while let Some((i, c)) = chars.next() {
        // Check if we're entering an HTML tag
        if c == '<' {
            in_tag = true;
            tag_content.clear();
            result.push(c);
            continue;
        }

        // Check if we're exiting an HTML tag
        if c == '>' && in_tag {
            in_tag = false;
            result.push(c);

            // Check tag type
            let tag_lower = tag_content.to_lowercase();

            // Code and pre tags
            if tag_lower.starts_with("code") || tag_lower.starts_with("pre") {
                in_code = true;
            } else if tag_lower.starts_with("/code") || tag_lower.starts_with("/pre") {
                in_code = false;
            }
            // Glossary span
            else if tag_lower.starts_with("span") && tag_lower.contains("glossary-term") {
                in_glossary_span = true;
            } else if tag_lower.starts_with("/span") && in_glossary_span {
                in_glossary_span = false;
            }
            // Anchor tags
            else if tag_lower.starts_with("a ") || tag_lower == "a" {
                in_anchor = true;
            } else if tag_lower.starts_with("/a") {
                in_anchor = false;
            }
            // Heading tags (h1-h6)
            else if tag_lower.starts_with('h') && tag_lower.len() >= 2 {
                let second_char = tag_lower.chars().nth(1);
                if matches!(second_char, Some('1'..='6')) {
                    // Check it's not just a prefix (e.g., "header")
                    let third_char = tag_lower.chars().nth(2);
                    if third_char.is_none() || !third_char.unwrap().is_alphabetic() {
                        in_heading = true;
                    }
                }
            } else if tag_lower.starts_with("/h") && tag_lower.len() >= 3 {
                let third_char = tag_lower.chars().nth(2);
                if matches!(third_char, Some('1'..='6')) {
                    in_heading = false;
                }
            }
            // Script tags
            else if tag_lower.starts_with("script") {
                in_script = true;
            } else if tag_lower.starts_with("/script") {
                in_script = false;
            }

            // no-glossary class detection (can be on any element)
            // Check for opening tags with no-glossary class
            if !tag_lower.starts_with('/') && tag_lower.contains("class=") && tag_lower.contains("no-glossary") {
                // Extract the tag name (first word before space or end)
                let tag_name = tag_lower.split_whitespace().next().unwrap_or("").to_string();
                if !tag_name.is_empty() {
                    no_glossary_stack.push(tag_name);
                }
            }
            // Track closing tags for no-glossary elements
            if tag_lower.starts_with('/') && !no_glossary_stack.is_empty() {
                // Extract closing tag name (remove leading /)
                let closing_tag = tag_lower.trim_start_matches('/').split_whitespace().next().unwrap_or("");
                // Pop from stack if it matches the most recent no-glossary element
                if let Some(last) = no_glossary_stack.last() {
                    if last == closing_tag {
                        no_glossary_stack.pop();
                    }
                }
            }

            continue;
        }

        // Collect tag content
        if in_tag {
            tag_content.push(c);
            result.push(c);
            continue;
        }

        // Skip replacement inside excluded elements
        if in_code || in_glossary_span || in_anchor || in_heading || in_script || !no_glossary_stack.is_empty() {
            result.push(c);
            continue;
        }

        // Check if the term starts here
        if html[i..].starts_with(term) {
            // Make sure it's a word boundary (not part of a larger word)
            let before_ok = i == 0 || !is_word_char(result.chars().last().unwrap_or(' '));
            let after_idx = i + term.len();
            let after_ok = after_idx >= html.len()
                || !is_word_char(html[after_idx..].chars().next().unwrap_or(' '));

            if before_ok && after_ok {
                // Escape definition for HTML attribute
                let escaped_def = html_escape_attribute(definition);
                result.push_str(&format!(
                    r#"<span class="glossary-term" data-definition="{}">{}</span>"#,
                    escaped_def, term
                ));

                // Skip the term characters
                for _ in 0..term.len() - 1 {
                    chars.next();
                }
                continue;
            }
        }

        result.push(c);
    }

    result
}

/// Check if a character is a word character (alphanumeric or Japanese)
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c > '\x7F'  // Japanese characters
}

/// Escape a string for use in an HTML attribute
fn html_escape_attribute(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_glossary() {
        let content = r#"# GLOSSARY

## API
Application Programming Interface の略

## SDK
Software Development Kit の略
"#;

        let glossary = Glossary::parse(content).unwrap();
        assert_eq!(glossary.entries.len(), 2);
        assert_eq!(
            glossary.get("API"),
            Some(&"Application Programming Interface の略".to_string())
        );
        assert_eq!(
            glossary.get("SDK"),
            Some(&"Software Development Kit の略".to_string())
        );
    }

    #[test]
    fn test_parse_multiline_definition() {
        let content = r#"# GLOSSARY

## REST
Representational State Transfer の略。
Web APIの設計スタイルの一つ。
"#;

        let glossary = Glossary::parse(content).unwrap();
        assert_eq!(
            glossary.get("REST"),
            Some(&"Representational State Transfer の略。 Web APIの設計スタイルの一つ。".to_string())
        );
    }

    #[test]
    fn test_apply_glossary() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = "<p>This is an API example.</p>";
        let result = apply_glossary(html, &glossary);
        assert!(result.contains(r#"<span class="glossary-term" data-definition="Interface">API</span>"#));
    }

    #[test]
    fn test_apply_glossary_in_code() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = "<p>Use the <code>API</code> endpoint.</p>";
        let result = apply_glossary(html, &glossary);
        // API inside code should not be wrapped
        assert!(result.contains("<code>API</code>"));
        assert!(!result.contains("glossary-term"));
    }

    #[test]
    fn test_apply_glossary_word_boundary() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = "<p>The APIARY tool is different from API.</p>";
        let result = apply_glossary(html, &glossary);
        // APIARY should not be affected
        assert!(result.contains("APIARY"));
        // But standalone API should be wrapped
        assert!(result.contains("glossary-term"));
    }

    #[test]
    fn test_empty_glossary() {
        let glossary = Glossary::default();
        assert!(glossary.is_empty());
    }

    #[test]
    fn test_sorted_terms_longest_first() {
        let content = r#"# GLOSSARY

## API
Application Programming Interface

## REST API
RESTful API

## REST
Representational State Transfer
"#;

        let glossary = Glossary::parse(content).unwrap();
        // REST API should come before REST and API
        assert_eq!(glossary.sorted_terms[0], "REST API");
    }

    #[test]
    fn test_apply_glossary_in_anchor() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = r#"<p>See <a href="/api">API documentation</a> for more info about API.</p>"#;
        let result = apply_glossary(html, &glossary);
        // API inside anchor should not be wrapped
        assert!(result.contains(">API documentation</a>"));
        // But standalone API outside anchor should be wrapped
        assert!(result.contains(r#"<span class="glossary-term" data-definition="Interface">API</span>.</p>"#));
    }

    #[test]
    fn test_apply_glossary_in_heading() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = "<h1>API Overview</h1><p>Learn about API.</p>";
        let result = apply_glossary(html, &glossary);
        // API inside h1 should not be wrapped
        assert!(result.contains("<h1>API Overview</h1>"));
        // But API in paragraph should be wrapped
        assert!(result.contains(r#"<span class="glossary-term" data-definition="Interface">API</span>"#));
    }

    #[test]
    fn test_apply_glossary_in_all_headings() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();

        // Test h1 through h6
        for level in 1..=6 {
            let html = format!("<h{}>API</h{}>", level, level);
            let result = apply_glossary(&html, &glossary);
            assert!(!result.contains("glossary-term"), "h{} should exclude glossary", level);
            assert!(result.contains(&format!("<h{}>API</h{}>", level, level)));
        }
    }

    #[test]
    fn test_apply_glossary_in_script() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = r#"<script>const API = "test";</script><p>Use the API.</p>"#;
        let result = apply_glossary(html, &glossary);
        // API inside script should not be wrapped
        assert!(result.contains(r#"<script>const API = "test";</script>"#));
        // But API in paragraph should be wrapped
        assert!(result.contains(r#"<span class="glossary-term" data-definition="Interface">API</span>"#));
    }

    #[test]
    fn test_apply_glossary_no_glossary_class() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = r#"<p>About API.</p><div class="no-glossary">API is excluded here.</div><p>API again.</p>"#;
        let result = apply_glossary(html, &glossary);
        // API inside no-glossary div should not be wrapped
        assert!(result.contains(r#"<div class="no-glossary">API is excluded here.</div>"#));
        // But API outside should be wrapped (count occurrences)
        let glossary_count = result.matches("glossary-term").count();
        assert_eq!(glossary_count, 2, "Should have 2 glossary terms (before and after no-glossary)");
    }

    #[test]
    fn test_apply_glossary_no_glossary_nested() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = r#"<div class="no-glossary"><p>API in <span>nested API</span> element.</p></div><p>API outside.</p>"#;
        let result = apply_glossary(html, &glossary);
        // API inside no-glossary (even nested) should not be wrapped
        assert!(result.contains(r#"<div class="no-glossary"><p>API in <span>nested API</span>"#));
        // But API outside should be wrapped
        assert!(result.contains(r#"<span class="glossary-term" data-definition="Interface">API</span> outside"#));
    }

    #[test]
    fn test_apply_glossary_header_not_matched() {
        // Ensure "header" element is not confused with "h1"-"h6"
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = "<header>API in header</header><p>API in p.</p>";
        let result = apply_glossary(html, &glossary);
        // API inside header element should still be wrapped (header != h1-h6)
        let glossary_count = result.matches("glossary-term").count();
        assert_eq!(glossary_count, 2, "Both API occurrences should be wrapped");
    }

    #[test]
    fn test_apply_glossary_anchor_with_attributes() {
        let glossary = Glossary::parse("## API\nInterface").unwrap();
        let html = r#"<a href="/doc" class="link" target="_blank">API Guide</a> and API."#;
        let result = apply_glossary(html, &glossary);
        // API inside anchor with attributes should not be wrapped
        assert!(result.contains(">API Guide</a>"));
        // API outside should be wrapped
        assert!(result.contains(r#"<span class="glossary-term" data-definition="Interface">API</span>."#));
    }
}
