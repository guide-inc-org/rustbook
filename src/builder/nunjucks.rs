//! Nunjucks-compatible template processing using Tera
//!
//! This module provides full Nunjucks/Jinja2 template syntax support for Markdown content,
//! including conditionals, loops, and filters.
//!
//! ## Supported Features
//!
//! ### Conditionals
//! ```text
//! {% if condition %}
//!   content
//! {% elif other_condition %}
//!   other content
//! {% else %}
//!   fallback
//! {% endif %}
//! ```
//!
//! ### Loops
//! ```text
//! {% for item in list %}
//!   {{ item }}
//! {% endfor %}
//! ```
//!
//! ### Filters
//! ```text
//! {{ value | upper }}
//! {{ value | lower }}
//! {{ value | default("fallback") }}
//! ```

use crate::parser::BookConfig;
use anyhow::{Context, Result};
use regex::Regex;
use tera::{Context as TeraContext, Tera};

/// Process Nunjucks templates in Markdown content
///
/// This function replaces the simple `expand_variables()` approach with full Tera template
/// processing, supporting conditionals, loops, and filters while maintaining backward
/// compatibility with `{{ book.xxx }}` syntax.
///
/// # Arguments
/// * `content` - The Markdown content containing Nunjucks templates
/// * `config` - Book configuration containing variables
///
/// # Returns
/// * `Ok(String)` - Processed content with templates rendered
/// * `Err` - Template parsing or rendering error with location info
pub fn process_nunjucks_templates(content: &str, config: &BookConfig) -> Result<String> {
    // Fast path: if no template syntax detected, return as-is
    if !has_template_syntax(content) {
        return Ok(content.to_string());
    }

    // Find protected regions (code blocks) to exclude from template processing
    let protected_regions = find_protected_regions(content);

    // If content has protected regions, we need to handle them specially
    if !protected_regions.is_empty() {
        return process_with_protected_regions(content, config, &protected_regions);
    }

    // No protected regions, process the entire content
    render_template(content, config)
}

/// Check if content contains any Nunjucks template syntax
fn has_template_syntax(content: &str) -> bool {
    // Quick check for common template markers
    content.contains("{{") || content.contains("{%")
}

/// Find all protected regions in the content (fenced code blocks)
/// These regions should not have template processing applied
fn find_protected_regions(content: &str) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();

    // Find fenced code blocks (``` ... ```)
    // Use a more robust approach that handles multi-line content
    let fenced_re = Regex::new(r"(?m)^```[^\n]*\n[\s\S]*?^```").unwrap();
    for m in fenced_re.find_iter(content) {
        regions.push((m.start(), m.end()));
    }

    // Also handle indented code blocks (4 spaces or tab at start)
    // These are less common but should be protected too
    // Note: This is a simplified check; full markdown parsing would be more accurate

    regions
}

/// Process content with protected regions
/// Splits content into protected and unprotected segments, only processing unprotected ones
fn process_with_protected_regions(
    content: &str,
    config: &BookConfig,
    protected_regions: &[(usize, usize)],
) -> Result<String> {
    let mut result = String::new();
    let mut last_end = 0;

    for (start, end) in protected_regions {
        // Process the unprotected segment before this code block
        if *start > last_end {
            let segment = &content[last_end..*start];
            let processed = render_template(segment, config)
                .with_context(|| format!("Template error in content before position {}", start))?;
            result.push_str(&processed);
        }

        // Add the protected region (code block) as-is
        result.push_str(&content[*start..*end]);
        last_end = *end;
    }

    // Process any remaining content after the last protected region
    if last_end < content.len() {
        let segment = &content[last_end..];
        let processed = render_template(segment, config)
            .with_context(|| "Template error in content after last code block")?;
        result.push_str(&processed);
    }

    Ok(result)
}

/// Render a template string using Tera
fn render_template(content: &str, config: &BookConfig) -> Result<String> {
    let mut tera = Tera::default();

    // Add custom template with a unique name
    tera.add_raw_template("__content__", content)
        .with_context(|| format_template_error(content, "Failed to parse template"))?;

    // Build context from book config
    let mut context = TeraContext::new();

    // Add all variables from book.json to context
    // They're accessible both as top-level and under "book" object
    for (key, value) in &config.variables {
        // Add as top-level variable
        context.insert(key, &json_to_tera_value(value));
    }

    // Add a "book" object for {{ book.xxx }} compatibility
    // This maintains backward compatibility with the existing syntax
    let book_map: std::collections::HashMap<String, tera::Value> = config
        .variables
        .iter()
        .map(|(k, v)| (k.clone(), json_to_tera_value(v)))
        .collect();
    context.insert("book", &book_map);

    // Render the template
    tera.render("__content__", &context)
        .with_context(|| format_template_error(content, "Failed to render template"))
}

/// Convert serde_json::Value to tera::Value
fn json_to_tera_value(json: &serde_json::Value) -> tera::Value {
    match json {
        serde_json::Value::Null => tera::Value::Null,
        serde_json::Value::Bool(b) => tera::Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                tera::Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                tera::Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into()).into())
            } else {
                tera::Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => tera::Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            let values: Vec<tera::Value> = arr.iter().map(json_to_tera_value).collect();
            tera::Value::Array(values)
        }
        serde_json::Value::Object(obj) => {
            let map: tera::Map<String, tera::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_tera_value(v)))
                .collect();
            tera::Value::Object(map)
        }
    }
}

/// Format template error with helpful context
fn format_template_error(content: &str, message: &str) -> String {
    // Try to find the problematic line
    let lines: Vec<&str> = content.lines().collect();
    let preview_lines = lines.iter().take(5).cloned().collect::<Vec<_>>().join("\n");

    format!(
        "{}\n\nContent preview:\n{}{}",
        message,
        preview_lines,
        if lines.len() > 5 { "\n..." } else { "" }
    )
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

    // === Basic Variable Tests ===

    #[test]
    fn test_basic_variable_expansion() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("1.0.0"));
        vars.insert("author".to_string(), serde_json::json!("Guide Inc"));

        let config = create_test_config(vars);
        let content = "Version: {{ book.version }}\nAuthor: {{ book.author }}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Version: 1.0.0\nAuthor: Guide Inc");
    }

    #[test]
    fn test_variable_without_spaces() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("2.0.0"));

        let config = create_test_config(vars);
        let content = "Version: {{book.version}}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Version: 2.0.0");
    }

    #[test]
    fn test_number_variable() {
        let mut vars = HashMap::new();
        vars.insert("year".to_string(), serde_json::json!(2024));

        let config = create_test_config(vars);
        let content = "Year: {{ book.year }}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Year: 2024");
    }

    #[test]
    fn test_boolean_variable() {
        let mut vars = HashMap::new();
        vars.insert("published".to_string(), serde_json::json!(true));

        let config = create_test_config(vars);
        let content = "Published: {{ book.published }}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Published: true");
    }

    // === Conditional Tests ===

    #[test]
    fn test_if_condition_true() {
        let mut vars = HashMap::new();
        vars.insert("show_feature".to_string(), serde_json::json!(true));

        let config = create_test_config(vars);
        let content = "{% if book.show_feature %}Feature is enabled{% endif %}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Feature is enabled");
    }

    #[test]
    fn test_if_condition_false() {
        let mut vars = HashMap::new();
        vars.insert("show_feature".to_string(), serde_json::json!(false));

        let config = create_test_config(vars);
        let content = "{% if book.show_feature %}Feature is enabled{% endif %}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_if_else_condition() {
        let mut vars = HashMap::new();
        vars.insert("premium".to_string(), serde_json::json!(false));

        let config = create_test_config(vars);
        let content = "{% if book.premium %}Premium content{% else %}Free content{% endif %}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Free content");
    }

    #[test]
    fn test_if_elif_else_condition() {
        let mut vars = HashMap::new();
        vars.insert("tier".to_string(), serde_json::json!("pro"));

        let config = create_test_config(vars);
        let content = r#"{% if book.tier == "basic" %}Basic{% elif book.tier == "pro" %}Professional{% else %}Enterprise{% endif %}"#;
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Professional");
    }

    // === Loop Tests ===

    #[test]
    fn test_for_loop_array() {
        let mut vars = HashMap::new();
        vars.insert(
            "features".to_string(),
            serde_json::json!(["Search", "Export", "Share"]),
        );

        let config = create_test_config(vars);
        let content = "Features:\n{% for feature in book.features %}- {{ feature }}\n{% endfor %}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Features:\n- Search\n- Export\n- Share\n");
    }

    #[test]
    fn test_for_loop_with_index() {
        let mut vars = HashMap::new();
        vars.insert(
            "items".to_string(),
            serde_json::json!(["A", "B", "C"]),
        );

        let config = create_test_config(vars);
        let content = "{% for item in book.items %}{{ loop.index }}. {{ item }}\n{% endfor %}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "1. A\n2. B\n3. C\n");
    }

    #[test]
    fn test_for_loop_empty_array() {
        let mut vars = HashMap::new();
        vars.insert("items".to_string(), serde_json::json!([]));

        let config = create_test_config(vars);
        let content = "{% for item in book.items %}{{ item }}{% endfor %}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "");
    }

    // === Filter Tests ===

    #[test]
    fn test_upper_filter() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("guide"));

        let config = create_test_config(vars);
        let content = "{{ book.name | upper }}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "GUIDE");
    }

    #[test]
    fn test_lower_filter() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("GUIDE"));

        let config = create_test_config(vars);
        let content = "{{ book.name | lower }}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "guide");
    }

    #[test]
    fn test_default_filter_with_value() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("Guide"));

        let config = create_test_config(vars);
        let content = r#"{{ book.name | default(value="Unknown") }}"#;
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Guide");
    }

    #[test]
    fn test_default_filter_without_value() {
        let vars = HashMap::new();

        let config = create_test_config(vars);
        let content = r#"{{ book.name | default(value="Unknown") }}"#;
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Unknown");
    }

    #[test]
    fn test_capitalize_filter() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("guide inc"));

        let config = create_test_config(vars);
        let content = "{{ book.name | capitalize }}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Guide inc");
    }

    #[test]
    fn test_length_filter() {
        let mut vars = HashMap::new();
        vars.insert(
            "items".to_string(),
            serde_json::json!(["a", "b", "c"]),
        );

        let config = create_test_config(vars);
        let content = "{{ book.items | length }}";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "3");
    }

    // === Code Block Protection Tests ===

    #[test]
    fn test_preserve_code_block() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("1.0.0"));

        let config = create_test_config(vars);
        let content = r#"Version: {{ book.version }}

```javascript
const template = "{{ book.version }}";
```

End"#;
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert!(result.contains("Version: 1.0.0"));
        assert!(result.contains(r#""{{ book.version }}""#));
        assert!(result.contains("End"));
    }

    #[test]
    fn test_multiple_code_blocks() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("Test"));

        let config = create_test_config(vars);
        let content = r#"Name: {{ book.name }}

```
{{ book.name }} in code
```

Middle: {{ book.name }}

```rust
let x = "{{ book.name }}";
```

End"#;
        let result = process_nunjucks_templates(content, &config).unwrap();

        // Outside code blocks should be expanded
        assert!(result.contains("Name: Test"));
        assert!(result.contains("Middle: Test"));
        // Inside code blocks should be preserved
        assert!(result.contains("{{ book.name }} in code"));
        assert!(result.contains(r#""{{ book.name }}""#));
    }

    // === Edge Cases ===

    #[test]
    fn test_no_template_syntax() {
        let config = create_test_config(HashMap::new());
        let content = "This is plain markdown without any template syntax.";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, content);
    }

    #[test]
    fn test_empty_content() {
        let config = create_test_config(HashMap::new());
        let content = "";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_nested_objects() {
        let mut vars = HashMap::new();
        vars.insert(
            "author".to_string(),
            serde_json::json!({
                "name": "John Doe",
                "email": "john@example.com"
            }),
        );

        let config = create_test_config(vars);
        let content = "Author: {{ book.author.name }} <{{ book.author.email }}>";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "Author: John Doe <john@example.com>");
    }

    // === Compatibility Tests ===

    #[test]
    fn test_top_level_variable_access() {
        // Variables should be accessible both as book.xxx and just xxx
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("1.0.0"));

        let config = create_test_config(vars);

        // Both syntaxes should work
        let content1 = "{{ book.version }}";
        let content2 = "{{ version }}";

        let result1 = process_nunjucks_templates(content1, &config).unwrap();
        let result2 = process_nunjucks_templates(content2, &config).unwrap();

        assert_eq!(result1, "1.0.0");
        assert_eq!(result2, "1.0.0");
    }

    // === Complex Markdown Tests ===

    #[test]
    fn test_template_in_markdown_heading() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), serde_json::json!("2.0"));

        let config = create_test_config(vars);
        let content = "# Guide v{{ book.version }}\n\nWelcome to version {{ book.version }}.";
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert_eq!(result, "# Guide v2.0\n\nWelcome to version 2.0.");
    }

    #[test]
    fn test_conditional_markdown_sections() {
        let mut vars = HashMap::new();
        vars.insert("show_advanced".to_string(), serde_json::json!(true));

        let config = create_test_config(vars);
        let content = r#"## Basic Usage

This is basic content.

{% if book.show_advanced %}
## Advanced Usage

This is advanced content.
{% endif %}"#;
        let result = process_nunjucks_templates(content, &config).unwrap();

        assert!(result.contains("## Basic Usage"));
        assert!(result.contains("## Advanced Usage"));
        assert!(result.contains("This is advanced content."));
    }
}
