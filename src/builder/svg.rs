//! SVG processing utilities for HTML optimization
//!
//! This module provides two main functions:
//! - `externalize_inline_svg`: Extracts inline SVGs to separate files for better caching
//! - `inline_svg_files`: Inlines SVG files into HTML for fewer HTTP requests
//!
//! Icon SVGs (with `fill="currentColor"`) are skipped to preserve their dynamic behavior.

use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::Path;

/// Check if an SVG is an icon (has fill="currentColor")
/// Icon SVGs should be kept inline to preserve their dynamic color behavior
fn is_icon_svg(svg_content: &str) -> bool {
    svg_content.contains(r#"fill="currentColor""#)
        || svg_content.contains(r#"fill='currentColor'"#)
}

/// Generate a unique filename for an externalized SVG
fn generate_svg_filename(index: usize, output_dir: &Path) -> String {
    let svg_dir = output_dir.join("assets").join("svg");
    let filename = format!("inline-{}.svg", index);

    // Ensure the directory exists
    let _ = fs::create_dir_all(&svg_dir);

    format!("assets/svg/{}", filename)
}

/// Externalize inline SVGs to separate files
///
/// Finds all inline `<svg>...</svg>` elements in the HTML, writes them to separate files,
/// and replaces them with `<img src="...">` tags.
///
/// SVGs with `fill="currentColor"` (icon SVGs) are skipped to preserve their dynamic behavior.
///
/// # Arguments
/// * `html` - The HTML content to process
/// * `output_dir` - The directory where SVG files will be written
///
/// # Returns
/// The modified HTML with inline SVGs replaced by img tags
pub fn externalize_inline_svg(html: &str, output_dir: &Path) -> Result<String> {
    // Regex to match inline SVG elements
    // Using (?s) flag for dotall mode to match across newlines
    let svg_regex = Regex::new(r"(?s)<svg([^>]*)>(.*?)</svg>")?;

    let mut result = html.to_string();
    let mut svg_index = 0;
    let mut offset: i64 = 0;

    for caps in svg_regex.captures_iter(html) {
        let full_match = caps.get(0).unwrap();
        let svg_attrs = &caps[1];
        let svg_inner = &caps[2];

        // Reconstruct full SVG content
        let svg_content = format!("<svg{}>{}</svg>", svg_attrs, svg_inner);

        // Skip icon SVGs
        if is_icon_svg(&svg_content) {
            continue;
        }

        // Generate filename and path
        let relative_path = generate_svg_filename(svg_index, output_dir);
        let svg_file_path = output_dir.join(&relative_path);

        // Ensure parent directory exists
        if let Some(parent) = svg_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write SVG content to file
        fs::write(&svg_file_path, &svg_content)?;

        // Extract width and height from SVG attributes if present
        let width_regex = Regex::new(r#"width\s*=\s*["']([^"']+)["']"#)?;
        let height_regex = Regex::new(r#"height\s*=\s*["']([^"']+)["']"#)?;

        let width = width_regex.captures(svg_attrs)
            .map(|c| c[1].to_string());
        let height = height_regex.captures(svg_attrs)
            .map(|c| c[1].to_string());

        // Build replacement img tag
        let mut img_tag = format!(r#"<img src="{}""#, relative_path);
        if let Some(w) = width {
            img_tag.push_str(&format!(r#" width="{}""#, w));
        }
        if let Some(h) = height {
            img_tag.push_str(&format!(r#" height="{}""#, h));
        }
        img_tag.push_str(r#" alt="SVG image">"#);

        // Calculate adjusted positions
        let start = (full_match.start() as i64 + offset) as usize;
        let end = (full_match.end() as i64 + offset) as usize;

        // Replace in result
        result.replace_range(start..end, &img_tag);

        // Update offset
        offset += img_tag.len() as i64 - (full_match.end() - full_match.start()) as i64;
        svg_index += 1;
    }

    Ok(result)
}

/// Inline SVG files into HTML
///
/// Finds all `<img src="...svg">` tags and replaces them with the inline SVG content.
/// This reduces HTTP requests by embedding SVGs directly in the HTML.
///
/// # Arguments
/// * `html` - The HTML content to process
/// * `base_dir` - The base directory for resolving relative SVG paths
///
/// # Returns
/// The modified HTML with img tags replaced by inline SVGs
pub fn inline_svg_files(html: &str, base_dir: &Path) -> Result<String> {
    // Regex to match img tags with SVG sources
    let img_regex = Regex::new(r#"<img([^>]+)src\s*=\s*["']([^"']+\.svg)["']([^>]*)>"#)?;

    let mut result = html.to_string();
    let mut offset: i64 = 0;

    for caps in img_regex.captures_iter(html) {
        let full_match = caps.get(0).unwrap();
        let before_src = &caps[1];
        let svg_path = &caps[2];
        let after_src = &caps[3];

        // Resolve the SVG file path
        let svg_file_path = base_dir.join(svg_path);

        // Read SVG content if file exists
        let svg_content = match fs::read_to_string(&svg_file_path) {
            Ok(content) => content,
            Err(_) => {
                // File not found, skip this replacement
                continue;
            }
        };

        // Skip icon SVGs (keep as img tags)
        if is_icon_svg(&svg_content) {
            continue;
        }

        // Extract width and height from img tag attributes
        let width_regex = Regex::new(r#"width\s*=\s*["']([^"']+)["']"#)?;
        let height_regex = Regex::new(r#"height\s*=\s*["']([^"']+)["']"#)?;

        let attrs = format!("{}{}", before_src, after_src);
        let width = width_regex.captures(&attrs)
            .map(|c| c[1].to_string());
        let height = height_regex.captures(&attrs)
            .map(|c| c[1].to_string());

        // Modify SVG to include width/height if specified in img tag
        let mut modified_svg = svg_content.clone();
        if let Some(w) = width {
            if !modified_svg.contains("width=") {
                modified_svg = modified_svg.replacen("<svg", &format!(r#"<svg width="{}""#, w), 1);
            }
        }
        if let Some(h) = height {
            if !modified_svg.contains("height=") {
                modified_svg = modified_svg.replacen("<svg", &format!(r#"<svg height="{}""#, h), 1);
            }
        }

        // Calculate adjusted positions
        let start = (full_match.start() as i64 + offset) as usize;
        let end = (full_match.end() as i64 + offset) as usize;

        // Replace in result
        result.replace_range(start..end, &modified_svg);

        // Update offset
        offset += modified_svg.len() as i64 - (full_match.end() - full_match.start()) as i64;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_is_icon_svg() {
        assert!(is_icon_svg(r#"<svg fill="currentColor"><path/></svg>"#));
        assert!(is_icon_svg(r#"<svg fill='currentColor'><path/></svg>"#));
        assert!(!is_icon_svg(r#"<svg fill="blue"><path/></svg>"#));
        assert!(!is_icon_svg(r#"<svg><path/></svg>"#));
    }

    #[test]
    fn test_externalize_inline_svg() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path();

        let html = r#"<html><body>
<svg width="100" height="100"><circle cx="50" cy="50" r="40"/></svg>
<p>Some text</p>
</body></html>"#;

        let result = externalize_inline_svg(html, output_dir).unwrap();

        // Should replace SVG with img tag
        assert!(result.contains(r#"<img src="assets/svg/inline-0.svg""#));
        assert!(result.contains(r#"width="100""#));
        assert!(result.contains(r#"height="100""#));
        assert!(!result.contains("<circle"));

        // SVG file should be created
        let svg_file = output_dir.join("assets/svg/inline-0.svg");
        assert!(svg_file.exists());
        let svg_content = fs::read_to_string(svg_file).unwrap();
        assert!(svg_content.contains("<circle"));
    }

    #[test]
    fn test_externalize_skips_icon_svg() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path();

        let html = r#"<html><body>
<svg fill="currentColor"><path d="M10 10"/></svg>
</body></html>"#;

        let result = externalize_inline_svg(html, output_dir).unwrap();

        // Icon SVG should remain inline
        assert!(result.contains(r#"fill="currentColor""#));
        assert!(result.contains("<svg"));
        assert!(!result.contains("<img"));
    }

    #[test]
    fn test_inline_svg_files() {
        let temp_dir = tempdir().unwrap();
        let base_dir = temp_dir.path();

        // Create a test SVG file
        let svg_content = r#"<svg viewBox="0 0 100 100"><rect width="100" height="100"/></svg>"#;
        fs::write(base_dir.join("test.svg"), svg_content).unwrap();

        let html = r#"<html><body>
<img src="test.svg" alt="Test">
</body></html>"#;

        let result = inline_svg_files(html, base_dir).unwrap();

        // Should inline the SVG
        assert!(result.contains("<svg viewBox"));
        assert!(result.contains("<rect"));
        assert!(!result.contains("<img"));
    }

    #[test]
    fn test_inline_svg_skips_icon() {
        let temp_dir = tempdir().unwrap();
        let base_dir = temp_dir.path();

        // Create an icon SVG file
        let svg_content = r#"<svg fill="currentColor"><path d="M10 10"/></svg>"#;
        fs::write(base_dir.join("icon.svg"), svg_content).unwrap();

        let html = r#"<html><body>
<img src="icon.svg" alt="Icon">
</body></html>"#;

        let result = inline_svg_files(html, base_dir).unwrap();

        // Icon SVG should remain as img tag
        assert!(result.contains("<img"));
        assert!(result.contains(r#"src="icon.svg""#));
        assert!(!result.contains(r#"fill="currentColor""#));
    }

    #[test]
    fn test_inline_svg_missing_file() {
        let temp_dir = tempdir().unwrap();
        let base_dir = temp_dir.path();

        let html = r#"<html><body>
<img src="nonexistent.svg" alt="Missing">
</body></html>"#;

        let result = inline_svg_files(html, base_dir).unwrap();

        // Should keep img tag unchanged when file doesn't exist
        assert!(result.contains("<img"));
        assert!(result.contains("nonexistent.svg"));
    }

    #[test]
    fn test_externalize_multiple_svgs() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path();

        let html = r#"<html><body>
<svg id="svg1"><circle r="10"/></svg>
<p>Text between</p>
<svg id="svg2"><rect width="20"/></svg>
</body></html>"#;

        let result = externalize_inline_svg(html, output_dir).unwrap();

        // Both SVGs should be externalized
        assert!(result.contains("inline-0.svg"));
        assert!(result.contains("inline-1.svg"));
        assert!(!result.contains("<circle"));
        assert!(!result.contains("<rect"));
    }
}
