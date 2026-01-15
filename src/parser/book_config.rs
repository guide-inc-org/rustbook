use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Plugins that are enabled by default (unless explicitly disabled with "-plugin-name")
const DEFAULT_ENABLED_PLUGINS: &[&str] = &[
    "collapsible-chapters",
    "back-to-top-button",
    "mermaid-md-adoc",
    "fontsettings",
];

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BookConfig {
    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub plugins: Vec<String>,

    #[serde(default)]
    pub styles: HashMap<String, String>,

    /// User-defined variables that can be used in Markdown with {{ book.xxx }} syntax
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,

    /// When true, treat single newlines as hard breaks (<br>)
    /// This makes trailing spaces unnecessary for line breaks
    #[serde(default)]
    pub hardbreaks: bool,

    /// When true, enable KaTeX math rendering
    /// Supports $...$ for inline math and $$...$$ for display math
    #[serde(default)]
    pub math: bool,

    /// When true, externalize inline SVGs to separate files for better caching
    /// Icon SVGs (with fill="currentColor") are kept inline
    #[serde(default)]
    pub externalize_svg: Option<bool>,

    /// When true, inline SVG files into HTML for fewer HTTP requests
    /// Icon SVGs (with fill="currentColor") are kept as img tags
    #[serde(default)]
    pub inline_svg: Option<bool>,

    /// When true, download remote (https://) images at build time for offline viewing
    /// Images are cached in _remote_images/ directory with CRC32-based filenames
    #[serde(default, rename = "fetchRemoteImages")]
    pub fetch_remote_images: bool,
}

impl BookConfig {
    pub fn load(book_dir: &Path) -> Result<Self> {
        let config_path = book_dir.join("book.json");

        if !config_path.exists() {
            // Create default book.json with all plugins enabled
            let default_json = r#"{
    "title": "My Book",
    "plugins": [
        "collapsible-chapters",
        "back-to-top-button",
        "mermaid-md-adoc",
        "fontsettings"
    ]
}
"#;
            fs::write(&config_path, default_json)?;
            println!("  Created default book.json");
        }

        let content = fs::read_to_string(&config_path)?;
        let config: BookConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Check if a plugin is enabled.
    /// - Explicitly disabled with "-plugin-name" → false
    /// - Explicitly enabled with "plugin-name" → true
    /// - In DEFAULT_ENABLED_PLUGINS → true
    /// - Otherwise → false
    pub fn is_plugin_enabled(&self, name: &str) -> bool {
        // Check if explicitly disabled
        if self.plugins.iter().any(|p| *p == format!("-{}", name)) {
            return false;
        }
        // Check if explicitly enabled
        if self.plugins.iter().any(|p| p == name) {
            return true;
        }
        // Check if default enabled
        DEFAULT_ENABLED_PLUGINS.contains(&name)
    }

    /// Get custom CSS path for website
    pub fn get_website_style(&self) -> Option<&String> {
        self.styles.get("website")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_book_json() {
        let json = r#"{
            "title": "Test Book",
            "plugins": ["back-to-top-button", "-search"],
            "styles": {
                "website": "styles/website.css"
            }
        }"#;

        let config: BookConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.title, "Test Book");
        assert!(config.is_plugin_enabled("back-to-top-button"));
        assert!(!config.is_plugin_enabled("search")); // Explicitly disabled with "-search"
        assert_eq!(config.get_website_style(), Some(&"styles/website.css".to_string()));
    }

    #[test]
    fn test_default_enabled_plugins() {
        // Empty plugins list - default plugins should still be enabled
        let json = r#"{"title": "Test"}"#;
        let config: BookConfig = serde_json::from_str(json).unwrap();

        assert!(config.is_plugin_enabled("collapsible-chapters"));
        assert!(config.is_plugin_enabled("back-to-top-button"));
        assert!(config.is_plugin_enabled("mermaid-md-adoc"));
        assert!(config.is_plugin_enabled("fontsettings"));
        // Non-default plugin should be disabled
        assert!(!config.is_plugin_enabled("some-other-plugin"));
    }

    #[test]
    fn test_explicitly_disable_default_plugin() {
        // Explicitly disable a default plugin
        let json = r#"{"plugins": ["-collapsible-chapters"]}"#;
        let config: BookConfig = serde_json::from_str(json).unwrap();

        assert!(!config.is_plugin_enabled("collapsible-chapters"));
        // Other default plugins should still be enabled
        assert!(config.is_plugin_enabled("back-to-top-button"));
        assert!(config.is_plugin_enabled("mermaid-md-adoc"));
        assert!(config.is_plugin_enabled("fontsettings"));
    }

    #[test]
    fn test_explicitly_disable_fontsettings() {
        // Explicitly disable fontsettings plugin
        let json = r#"{"plugins": ["-fontsettings"]}"#;
        let config: BookConfig = serde_json::from_str(json).unwrap();

        assert!(!config.is_plugin_enabled("fontsettings"));
        // Other default plugins should still be enabled
        assert!(config.is_plugin_enabled("back-to-top-button"));
        assert!(config.is_plugin_enabled("mermaid-md-adoc"));
        assert!(config.is_plugin_enabled("collapsible-chapters"));
    }

    #[test]
    fn test_parse_variables() {
        let json = r#"{
            "title": "Test Book",
            "variables": {
                "version": "1.0.0",
                "author": "Guide Inc",
                "year": 2024
            }
        }"#;

        let config: BookConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.variables.get("version").unwrap(), "1.0.0");
        assert_eq!(config.variables.get("author").unwrap(), "Guide Inc");
        assert_eq!(config.variables.get("year").unwrap(), 2024);
    }

    #[test]
    fn test_empty_variables() {
        let json = r#"{"title": "Test"}"#;
        let config: BookConfig = serde_json::from_str(json).unwrap();
        assert!(config.variables.is_empty());
    }

    #[test]
    fn test_math_enabled() {
        let json = r#"{"title": "Test", "math": true}"#;
        let config: BookConfig = serde_json::from_str(json).unwrap();
        assert!(config.math);
    }

    #[test]
    fn test_math_disabled_by_default() {
        let json = r#"{"title": "Test"}"#;
        let config: BookConfig = serde_json::from_str(json).unwrap();
        assert!(!config.math);
    }

    #[test]
    fn test_fetch_remote_images_enabled() {
        let json = r#"{"title": "Test", "fetchRemoteImages": true}"#;
        let config: BookConfig = serde_json::from_str(json).unwrap();
        assert!(config.fetch_remote_images);
    }

    #[test]
    fn test_fetch_remote_images_disabled_by_default() {
        let json = r#"{"title": "Test"}"#;
        let config: BookConfig = serde_json::from_str(json).unwrap();
        assert!(!config.fetch_remote_images);
    }
}
