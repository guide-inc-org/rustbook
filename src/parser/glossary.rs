use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Glossary entry containing term and its definition
#[derive(Debug, Clone)]
pub struct GlossaryEntry {
    pub term: String,
    pub definition: String,
}

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
/// - Already processed terms
fn replace_term_in_html(html: &str, term: &str, definition: &str) -> String {
    let mut result = String::new();
    let mut chars = html.char_indices().peekable();
    let mut in_tag = false;
    let mut in_code = false;
    let mut in_glossary_span = false;
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
            if tag_lower.starts_with("code") || tag_lower.starts_with("pre") {
                in_code = true;
            } else if tag_lower.starts_with("/code") || tag_lower.starts_with("/pre") {
                in_code = false;
            } else if tag_lower.starts_with("span") && tag_lower.contains("glossary-term") {
                in_glossary_span = true;
            } else if tag_lower.starts_with("/span") && in_glossary_span {
                in_glossary_span = false;
            }
            continue;
        }

        // Collect tag content
        if in_tag {
            tag_content.push(c);
            result.push(c);
            continue;
        }

        // Skip replacement inside code blocks or existing glossary spans
        if in_code || in_glossary_span {
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
}
