//! Front Matter parser for Markdown files
//!
//! Parses YAML front matter from markdown files in the format:
//! ```markdown
//! ---
//! title: Custom Title
//! description: Page description
//! ---
//!
//! # Content
//! ```

use serde::Deserialize;

/// Front matter metadata extracted from markdown files
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FrontMatter {
    /// Custom page title (overrides default from SUMMARY.md)
    #[serde(default)]
    pub title: Option<String>,

    /// Page description for meta tags
    #[serde(default)]
    pub description: Option<String>,

    /// Additional custom fields can be added here
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_yaml::Value>,
}

/// Result of parsing front matter from markdown content
#[derive(Debug)]
pub struct ParsedContent {
    /// The extracted front matter (if any)
    pub front_matter: Option<FrontMatter>,

    /// The remaining markdown content without front matter
    pub content: String,
}

/// Parse front matter from markdown content
///
/// Front matter must be at the very beginning of the file and enclosed by `---` delimiters.
///
/// # Examples
///
/// ```
/// use guidebook::parser::frontmatter::parse_front_matter;
///
/// let content = r#"---
/// title: My Page
/// description: This is my page
/// ---
///
/// # Hello World
/// "#;
///
/// let parsed = parse_front_matter(content);
/// assert!(parsed.front_matter.is_some());
/// let fm = parsed.front_matter.unwrap();
/// assert_eq!(fm.title.as_deref(), Some("My Page"));
/// assert_eq!(fm.description.as_deref(), Some("This is my page"));
/// ```
pub fn parse_front_matter(content: &str) -> ParsedContent {
    // Check if content starts with front matter delimiter
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return ParsedContent {
            front_matter: None,
            content: content.to_string(),
        };
    }

    // Find the end of the opening delimiter line
    let after_opening = match trimmed.strip_prefix("---") {
        Some(rest) => rest,
        None => {
            return ParsedContent {
                front_matter: None,
                content: content.to_string(),
            }
        }
    };

    // Skip any whitespace/newline after opening ---
    let after_opening = after_opening.trim_start_matches([' ', '\t']);
    let after_opening = if after_opening.starts_with('\n') {
        &after_opening[1..]
    } else if after_opening.starts_with("\r\n") {
        &after_opening[2..]
    } else if after_opening.is_empty() {
        after_opening
    } else {
        // Something unexpected after ---, not valid front matter
        return ParsedContent {
            front_matter: None,
            content: content.to_string(),
        };
    };

    // Find the closing ---
    // First, check if the content starts with --- (empty front matter case)
    let (yaml_content, remaining) = if after_opening.starts_with("---\n") {
        ("", &after_opening[4..])
    } else if after_opening.starts_with("---\r\n") {
        ("", &after_opening[5..])
    } else if after_opening == "---" {
        ("", "")
    } else {
        // Look for closing --- with newline patterns
        let closing_patterns = ["\n---\n", "\n---\r\n", "\r\n---\n", "\r\n---\r\n", "\n---"];

        let mut end_pos = None;
        let mut pattern_len = 0;

        for pattern in closing_patterns {
            if let Some(pos) = after_opening.find(pattern) {
                if end_pos.is_none() || pos < end_pos.unwrap() {
                    end_pos = Some(pos);
                    pattern_len = pattern.len();
                }
            }
        }

        match end_pos {
            Some(pos) => {
                let yaml = &after_opening[..pos];
                let after_closing = &after_opening[pos + pattern_len..];
                (yaml, after_closing)
            }
            None => {
                // No closing delimiter found
                return ParsedContent {
                    front_matter: None,
                    content: content.to_string(),
                };
            }
        }
    };

    // Parse the YAML content
    match serde_yaml::from_str::<FrontMatter>(yaml_content) {
        Ok(fm) => ParsedContent {
            front_matter: Some(fm),
            content: remaining.to_string(),
        },
        Err(_) => {
            // YAML parsing failed, return original content
            ParsedContent {
                front_matter: None,
                content: content.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_front_matter() {
        let content = r#"---
title: My Custom Title
description: This is a description
---

# Hello World

Some content here.
"#;
        let parsed = parse_front_matter(content);
        assert!(parsed.front_matter.is_some());

        let fm = parsed.front_matter.unwrap();
        assert_eq!(fm.title.as_deref(), Some("My Custom Title"));
        assert_eq!(fm.description.as_deref(), Some("This is a description"));
        assert!(parsed.content.contains("# Hello World"));
        assert!(!parsed.content.contains("My Custom Title"));
    }

    #[test]
    fn test_parse_without_front_matter() {
        let content = r#"# Hello World

Some content here.
"#;
        let parsed = parse_front_matter(content);
        assert!(parsed.front_matter.is_none());
        assert_eq!(parsed.content, content);
    }

    #[test]
    fn test_parse_with_only_title() {
        let content = r#"---
title: Only Title
---

Content
"#;
        let parsed = parse_front_matter(content);
        assert!(parsed.front_matter.is_some());

        let fm = parsed.front_matter.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Only Title"));
        assert!(fm.description.is_none());
    }

    #[test]
    fn test_parse_with_empty_front_matter() {
        let content = r#"---
---

Content
"#;
        let parsed = parse_front_matter(content);
        assert!(parsed.front_matter.is_some());

        let fm = parsed.front_matter.unwrap();
        assert!(fm.title.is_none());
        assert!(fm.description.is_none());
    }

    #[test]
    fn test_parse_with_extra_fields() {
        let content = r#"---
title: Test
author: John Doe
custom_field: value
---

Content
"#;
        let parsed = parse_front_matter(content);
        assert!(parsed.front_matter.is_some());

        let fm = parsed.front_matter.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Test"));
        assert!(fm.extra.contains_key("author"));
        assert!(fm.extra.contains_key("custom_field"));
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let content = r#"---
title: [invalid yaml
---

Content
"#;
        let parsed = parse_front_matter(content);
        // Invalid YAML should return original content
        assert!(parsed.front_matter.is_none());
        assert_eq!(parsed.content, content);
    }

    #[test]
    fn test_parse_not_at_start() {
        let content = r#"Some text first

---
title: Not Front Matter
---

Content
"#;
        let parsed = parse_front_matter(content);
        // Front matter must be at the start
        assert!(parsed.front_matter.is_none());
        assert_eq!(parsed.content, content);
    }

    #[test]
    fn test_parse_no_closing_delimiter() {
        let content = r#"---
title: No Closing

Content
"#;
        let parsed = parse_front_matter(content);
        assert!(parsed.front_matter.is_none());
        assert_eq!(parsed.content, content);
    }

    #[test]
    fn test_parse_japanese_content() {
        let content = r#"---
title: Japanese Title
description: Japanese description
---

# Japanese Content
"#;
        let parsed = parse_front_matter(content);
        assert!(parsed.front_matter.is_some());

        let fm = parsed.front_matter.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Japanese Title"));
        assert_eq!(fm.description.as_deref(), Some("Japanese description"));
    }
}
