//! Remote image downloading for offline viewing
//!
//! Downloads `https://` images at build time and replaces URLs in HTML
//! with local paths for offline access.

use crc32fast::Hasher;
use regex::Regex;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Downloads and caches remote images for offline viewing
pub struct ImageDownloader {
    client: Client,
    cache: HashMap<String, String>,
    #[allow(dead_code)]
    output_dir: PathBuf,
    images_dir: PathBuf,
}

impl ImageDownloader {
    /// Create a new ImageDownloader
    ///
    /// # Arguments
    /// * `output_dir` - The root output directory for the book build
    pub fn new(output_dir: &Path) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        let images_dir = output_dir.join("_remote_images");

        ImageDownloader {
            client,
            cache: HashMap::new(),
            output_dir: output_dir.to_path_buf(),
            images_dir,
        }
    }

    /// Process HTML content and download any remote images
    ///
    /// Finds all `<img src="https://...">` tags and downloads the images,
    /// replacing the URLs with local paths.
    ///
    /// # Arguments
    /// * `html` - The HTML content to process
    ///
    /// # Returns
    /// The HTML with remote image URLs replaced with local paths
    pub fn process_html(&mut self, html: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Regex to match img src attributes with https:// URLs
        let img_re = Regex::new(r#"<img\s+([^>]*?)src\s*=\s*["']((https?://[^"']+))["']([^>]*)>"#)?;

        let mut result = html.to_string();
        let mut replacements: Vec<(String, String)> = Vec::new();

        for caps in img_re.captures_iter(html) {
            let full_match = caps.get(0).unwrap().as_str();
            let before_src = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let url = caps.get(2).unwrap().as_str();
            let after_src = caps.get(4).map(|m| m.as_str()).unwrap_or("");

            // Only process https:// URLs
            if !url.starts_with("https://") && !url.starts_with("http://") {
                continue;
            }

            // Download the image and get local path
            match self.download_image(url) {
                Ok(local_path) => {
                    let new_tag = format!(
                        r#"<img {}src="{}"{}"#,
                        before_src, local_path, after_src
                    );
                    // Close the tag properly
                    let new_tag = if full_match.ends_with("/>") {
                        format!("{}/>", new_tag)
                    } else {
                        format!("{}>", new_tag)
                    };
                    replacements.push((full_match.to_string(), new_tag));
                }
                Err(e) => {
                    eprintln!("  Warning: Failed to download image {}: {}", url, e);
                    // Keep original URL on failure
                }
            }
        }

        // Apply replacements
        for (old, new) in replacements {
            result = result.replace(&old, &new);
        }

        Ok(result)
    }

    /// Download an image from a URL and return the local path
    fn download_image(&mut self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(cached_path) = self.cache.get(url) {
            return Ok(cached_path.clone());
        }

        // Create images directory if needed
        fs::create_dir_all(&self.images_dir)?;

        // Download the image
        let response = self.client.get(url).send()?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()).into());
        }

        let bytes = response.bytes()?;

        // Generate filename from URL hash + detected extension
        let hash = crc32_hash(url);
        let ext = detect_extension(url, &bytes);
        let filename = format!("{:08x}.{}", hash, ext);
        let file_path = self.images_dir.join(&filename);

        // Write the file
        fs::write(&file_path, &bytes)?;

        // Calculate relative path from output root
        let relative_path = format!("_remote_images/{}", filename);

        // Cache the result
        self.cache.insert(url.to_string(), relative_path.clone());

        Ok(relative_path)
    }

    /// Get download statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), 0) // (downloaded, failed)
    }
}

/// Calculate CRC32 hash of a string
fn crc32_hash(s: &str) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(s.as_bytes());
    hasher.finalize()
}

/// Detect image extension from URL or magic bytes
fn detect_extension(url: &str, bytes: &[u8]) -> &'static str {
    // Try to detect from magic bytes first
    if bytes.len() >= 8 {
        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return "png";
        }
        // JPEG: FF D8 FF
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return "jpg";
        }
        // GIF: GIF87a or GIF89a
        if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            return "gif";
        }
        // WebP: RIFF....WEBP
        if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
            return "webp";
        }
        // SVG: starts with <?xml or <svg
        let start = String::from_utf8_lossy(&bytes[..bytes.len().min(100)]);
        if start.trim_start().starts_with("<?xml") || start.trim_start().starts_with("<svg") {
            return "svg";
        }
        // ICO: 00 00 01 00
        if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            return "ico";
        }
        // BMP: BM
        if bytes.starts_with(b"BM") {
            return "bmp";
        }
    }

    // Fallback: try to get extension from URL
    let url_lower = url.to_lowercase();
    if let Some(ext_start) = url_lower.rfind('.') {
        let ext = &url_lower[ext_start + 1..];
        // Remove query parameters
        let ext = ext.split('?').next().unwrap_or(ext);
        let ext = ext.split('#').next().unwrap_or(ext);

        match ext {
            "png" => return "png",
            "jpg" | "jpeg" => return "jpg",
            "gif" => return "gif",
            "webp" => return "webp",
            "svg" => return "svg",
            "ico" => return "ico",
            "bmp" => return "bmp",
            "avif" => return "avif",
            _ => {}
        }
    }

    // Default to png if we can't determine
    "png"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_hash() {
        let hash1 = crc32_hash("https://example.com/image.png");
        let hash2 = crc32_hash("https://example.com/image.png");
        let hash3 = crc32_hash("https://example.com/other.png");

        assert_eq!(hash1, hash2, "Same input should produce same hash");
        assert_ne!(hash1, hash3, "Different input should produce different hash");
    }

    #[test]
    fn test_detect_extension_from_magic_bytes() {
        // PNG magic bytes
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(detect_extension("http://example.com/image", &png_bytes), "png");

        // JPEG magic bytes
        let jpg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(detect_extension("http://example.com/image", &jpg_bytes), "jpg");

        // GIF magic bytes
        let gif_bytes = b"GIF89a\x00\x00";
        assert_eq!(detect_extension("http://example.com/image", gif_bytes), "gif");
    }

    #[test]
    fn test_detect_extension_from_url() {
        let empty: &[u8] = &[];
        assert_eq!(detect_extension("https://example.com/image.png", empty), "png");
        assert_eq!(detect_extension("https://example.com/image.jpg", empty), "jpg");
        assert_eq!(detect_extension("https://example.com/image.jpeg", empty), "jpg");
        assert_eq!(detect_extension("https://example.com/image.gif", empty), "gif");
        assert_eq!(detect_extension("https://example.com/image.webp", empty), "webp");

        // With query parameters
        assert_eq!(
            detect_extension("https://example.com/image.png?v=123", empty),
            "png"
        );
    }

    #[test]
    fn test_detect_extension_default() {
        let empty: &[u8] = &[];
        // Unknown extension should default to png
        assert_eq!(detect_extension("https://example.com/image", empty), "png");
        assert_eq!(detect_extension("https://example.com/image.xyz", empty), "png");
    }
}
