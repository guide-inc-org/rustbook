mod parser;
mod builder;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tiny_http::{Server, Response, Header};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use notify::event::ModifyKind;
use percent_encoding::percent_decode_str;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "guidebook")]
#[command(version)]
#[command(about = "HonKit/GitBook compatible static book generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new book
    Init {
        /// Directory to initialize
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Build the book
    Build {
        /// Source directory
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Output directory
        #[arg(short, long, default_value = "_book")]
        output: PathBuf,
    },
    /// Start a local server for preview
    Serve {
        /// Source directory
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Port to listen on
        #[arg(short, long, default_value = "4000")]
        port: u16,
        /// Open browser automatically
        #[arg(short, long)]
        open: bool,
    },
    /// Update guidebook to the latest version
    Update,
}

fn main() -> Result<()> {
    // Check for updates in background (non-blocking)
    check_for_updates();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            init_book(&path)
        }
        Commands::Build { path, output } => {
            println!("Building book from {:?} to {:?}", path, output);
            builder::build(&path, &output)
        }
        Commands::Serve { path, port, open } => {
            serve_book(&path, port, open)
        }
        Commands::Update => {
            update_self()
        }
    }
}

fn init_book(path: &PathBuf) -> Result<()> {
    println!("Initializing book in {:?}", path);

    // Create directory if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(path)?;
        println!("  Created directory {:?}", path);
    }

    // Create README.md
    let readme_path = path.join("README.md");
    if !readme_path.exists() {
        let readme_content = r#"# Introduction

Welcome to your new book!

This file serves as your book's introduction or preface.
"#;
        fs::write(&readme_path, readme_content)?;
        println!("  Created README.md");
    } else {
        println!("  README.md already exists, skipping");
    }

    // Create SUMMARY.md
    let summary_path = path.join("SUMMARY.md");
    if !summary_path.exists() {
        let summary_content = r#"# Summary

* [Introduction](README.md)
"#;
        fs::write(&summary_path, summary_content)?;
        println!("  Created SUMMARY.md");
    } else {
        println!("  SUMMARY.md already exists, skipping");
    }

    // Create book.json
    let book_json_path = path.join("book.json");
    if !book_json_path.exists() {
        let book_json_content = r#"{
    "title": "My Book",
    "description": "",
    "author": "",
    "plugins": [
        "collapsible-chapters",
        "back-to-top-button",
        "mermaid-md-adoc"
    ]
}
"#;
        fs::write(&book_json_path, book_json_content)?;
        println!("  Created book.json");
    } else {
        println!("  book.json already exists, skipping");
    }

    println!("\nBook initialized successfully!");
    println!("\nNext steps:");
    println!("  1. Edit SUMMARY.md to define your book structure");
    println!("  2. Create markdown files for your chapters");
    println!("  3. Run 'guidebook serve' to preview your book");

    Ok(())
}

fn serve_book(source: &PathBuf, port: u16, open_browser: bool) -> Result<()> {
    // Build to temp directory
    let temp_dir = std::env::temp_dir().join("guidebook-serve");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }

    println!("Building book...");
    builder::build(source, &temp_dir)?;

    // Version counter for hot reload
    let version = Arc::new(AtomicU64::new(1));
    let version_for_watcher = version.clone();
    let source_for_watcher = source.clone();
    let temp_dir_for_watcher = temp_dir.clone();

    // Setup file watcher
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            // Only react to file modifications
            let dominated: bool = matches!(
                event.kind,
                EventKind::Modify(ModifyKind::Data(_)) |
                EventKind::Modify(ModifyKind::Name(_)) |
                EventKind::Create(_) |
                EventKind::Remove(_)
            );
            if dominated {
                // Check if it's a relevant file (md, json, css, js)
                // Exclude _book directory and other build artifacts
                let dominated = event.paths.iter().any(|p| {
                    // Skip files in _book directory (build output)
                    let path_str = p.to_string_lossy();
                    if path_str.contains("/_book/") || path_str.contains("\\_book\\") {
                        return false;
                    }
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| matches!(e, "md" | "json" | "css" | "js" | "html"))
                        .unwrap_or(false)
                });
                if dominated {
                    println!("\n🔄 File changed, rebuilding...");
                    // Skip search index generation on hot reload for performance
                    if let Err(e) = builder::build_with_options(&source_for_watcher, &temp_dir_for_watcher, true) {
                        eprintln!("   Build error: {}", e);
                    } else {
                        version_for_watcher.fetch_add(1, Ordering::SeqCst);
                        println!("   Rebuild complete!");
                    }
                }
            }
        }
    })?;

    watcher.watch(source, RecursiveMode::Recursive)?;

    let addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&addr).map_err(|e| {
        if e.to_string().contains("Address already in use") {
            anyhow::anyhow!(
                "Port {} is already in use.\n   Try: guidebook serve -p {}",
                port,
                port + 1
            )
        } else {
            anyhow::anyhow!("Failed to start server: {}", e)
        }
    })?;

    let url = format!("http://localhost:{}/", port);
    println!("\n📚 Serving book at {}", url);
    println!("   🔥 Hot reload enabled - changes will auto-refresh");
    println!("   Press Ctrl+C to stop\n");

    // Open browser if requested
    if open_browser {
        if let Err(e) = open::that(&url) {
            eprintln!("   Failed to open browser: {}", e);
        }
    }

    // Keep watcher alive
    let _watcher = watcher;

    for request in server.incoming_requests() {
        let url = request.url().to_string();

        // Handle livereload polling endpoint
        if url.starts_with("/__livereload") {
            // Extract version from query string
            let client_version: u64 = url
                .split("?v=")
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);

            let current_version = version.load(Ordering::SeqCst);

            // If versions differ, tell client to reload
            let response_body = if client_version < current_version {
                format!(r#"{{"reload":true,"version":{}}}"#, current_version)
            } else {
                format!(r#"{{"reload":false,"version":{}}}"#, current_version)
            };

            let header = Header::from_bytes("Content-Type", "application/json").unwrap();
            let response = Response::from_string(response_body).with_header(header);
            let _ = request.respond(response);
            continue;
        }

        let url_path = if url == "/" {
            "/index.html".to_string()
        } else if url.ends_with('/') {
            format!("{}index.html", url)
        } else {
            url.clone()
        };

        // URL decode the path to handle Japanese/special characters
        let decoded_path = percent_decode_str(&url_path)
            .decode_utf8_lossy()
            .to_string();
        let file_path = temp_dir.join(decoded_path.trim_start_matches('/'));

        if file_path.exists() && file_path.is_file() {
            let mut content = fs::read(&file_path).unwrap_or_default();
            let content_type = get_content_type(&file_path);

            // Inject livereload script into HTML pages
            if content_type.starts_with("text/html") {
                let current_version = version.load(Ordering::SeqCst);
                let livereload_script = format!(
                    r#"<script>
(function(){{
    var version={};
    function checkReload(){{
        fetch('/__livereload?v='+version)
            .then(function(r){{return r.json()}})
            .then(function(data){{
                if(data.reload){{
                    version=data.version;
                    location.reload();
                }}
            }})
            .catch(function(){{}});
    }}
    setInterval(checkReload,1000);
}})();
</script></body>"#,
                    current_version
                );
                let html = String::from_utf8_lossy(&content);
                let html = html.replace("</body>", &livereload_script);
                content = html.into_bytes();
            }

            let header = Header::from_bytes("Content-Type", content_type).unwrap();
            let response = Response::from_data(content).with_header(header);
            let _ = request.respond(response);
        } else {
            // Try with .html extension
            let html_path = format!("{}.html", file_path.display());
            let html_path = PathBuf::from(&html_path);
            if html_path.exists() {
                let content = fs::read(&html_path).unwrap_or_default();
                let header = Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap();
                let response = Response::from_data(content).with_header(header);
                let _ = request.respond(response);
            } else {
                let response = Response::from_string("404 Not Found").with_status_code(404);
                let _ = request.respond(response);
            }
        }
    }

    Ok(())
}

fn get_content_type(path: &PathBuf) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}

fn check_for_updates() {
    // Run in a separate thread to not block startup
    std::thread::spawn(|| {
        if let Some(latest) = get_latest_version() {
            if is_newer_version(&latest, VERSION) {
                eprintln!(
                    "\n📦 New version available: {} → {}\n   Run: cargo install guidebook --force\n",
                    VERSION, latest
                );
            }
        }
    });
}

fn get_latest_version() -> Option<String> {
    let response = ureq::get("https://crates.io/api/v1/crates/guidebook")
        .set("User-Agent", &format!("guidebook/{}", VERSION))
        .timeout(std::time::Duration::from_secs(2))
        .call()
        .ok()?;

    let body = response.into_string().ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    json["crate"]["max_version"]
        .as_str()
        .map(String::from)
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };

    let latest_parts = parse(latest);
    let current_parts = parse(current);

    for (l, c) in latest_parts.iter().zip(current_parts.iter()) {
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    latest_parts.len() > current_parts.len()
}

fn update_self() -> Result<()> {
    use std::io::{Read, Write};

    println!("Checking for updates...");

    // Get latest version from GitHub
    let latest_version = get_latest_github_version()
        .ok_or_else(|| anyhow::anyhow!("Failed to check latest version"))?;

    println!("  Current version: {}", VERSION);
    println!("  Latest version:  {}", latest_version);

    if !is_newer_version(&latest_version, VERSION) {
        println!("\nYou're already on the latest version!");
        return Ok(());
    }

    // Detect platform
    let artifact_name = get_artifact_name()
        .ok_or_else(|| anyhow::anyhow!("Unsupported platform"))?;

    println!("\nDownloading {}...", artifact_name);

    // Download from GitHub Releases
    let download_url = format!(
        "https://github.com/guide-inc-org/guidebook/releases/download/v{}/{}",
        latest_version, artifact_name
    );

    let response = ureq::get(&download_url)
        .set("User-Agent", &format!("guidebook/{}", VERSION))
        .call()
        .map_err(|e| anyhow::anyhow!("Failed to download: {}", e))?;

    // Read response body
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    // Get current executable path
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe.parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot get executable directory"))?;

    // Extract binary
    let new_binary = if artifact_name.ends_with(".zip") {
        extract_zip(&bytes)?
    } else {
        extract_tar_gz(&bytes)?
    };

    // Replace current executable
    let backup_path = exe_dir.join("guidebook.backup");
    let new_exe_path = exe_dir.join(if cfg!(windows) { "guidebook_new.exe" } else { "guidebook_new" });

    // Write new binary
    let mut file = fs::File::create(&new_exe_path)?;
    file.write_all(&new_binary)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&new_exe_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&new_exe_path, perms)?;
    }

    // Backup current executable
    if backup_path.exists() {
        fs::remove_file(&backup_path)?;
    }
    fs::rename(&current_exe, &backup_path)?;

    // Move new executable to current location
    fs::rename(&new_exe_path, &current_exe)?;

    // Remove backup
    let _ = fs::remove_file(&backup_path);

    println!("\nSuccessfully updated to v{}!", latest_version);
    Ok(())
}

fn get_latest_github_version() -> Option<String> {
    let response = ureq::get("https://api.github.com/repos/guide-inc-org/guidebook/releases/latest")
        .set("User-Agent", &format!("guidebook/{}", VERSION))
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .ok()?;

    let body = response.into_string().ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    json["tag_name"]
        .as_str()
        .map(|s| s.trim_start_matches('v').to_string())
}

fn get_artifact_name() -> Option<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => Some("guidebook-linux-x86_64.tar.gz"),
        ("macos", "x86_64") => Some("guidebook-darwin-x86_64.tar.gz"),
        ("macos", "aarch64") => Some("guidebook-darwin-arm64.tar.gz"),
        ("windows", "x86_64") => Some("guidebook-windows-x86_64.zip"),
        _ => None,
    }
}

fn extract_tar_gz(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use tar::Archive;
    use std::io::{Cursor, Read};

    let decoder = GzDecoder::new(Cursor::new(data));
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path.file_name().map(|n| n == "guidebook").unwrap_or(false) {
            let mut binary = Vec::new();
            entry.read_to_end(&mut binary)?;
            return Ok(binary);
        }
    }

    Err(anyhow::anyhow!("Binary not found in archive"))
}

fn extract_zip(data: &[u8]) -> Result<Vec<u8>> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.ends_with("guidebook.exe") || name == "guidebook.exe" {
            let mut binary = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut binary)?;
            return Ok(binary);
        }
    }

    Err(anyhow::anyhow!("Binary not found in archive"))
}
