use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BookConfig {
    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub author: String,

    #[serde(default)]
    pub plugins: Vec<String>,

    #[serde(default)]
    pub styles: HashMap<String, String>,

    #[serde(default, rename = "pluginsConfig")]
    pub plugins_config: HashMap<String, serde_json::Value>,
}

impl BookConfig {
    pub fn load(book_dir: &Path) -> Result<Self> {
        let config_path = book_dir.join("book.json");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: BookConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Check if a plugin is enabled (not prefixed with -)
    pub fn is_plugin_enabled(&self, name: &str) -> bool {
        self.plugins.iter().any(|p| p == name)
    }

    /// Check if a plugin is explicitly disabled (prefixed with -)
    pub fn is_plugin_disabled(&self, name: &str) -> bool {
        self.plugins.iter().any(|p| *p == format!("-{}", name))
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
        assert!(config.is_plugin_disabled("search"));
        assert_eq!(config.get_website_style(), Some(&"styles/website.css".to_string()));
    }
}
