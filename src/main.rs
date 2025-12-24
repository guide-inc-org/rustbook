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
    },
}

fn main() -> Result<()> {
    // Check for updates in background (non-blocking)
    check_for_updates();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            println!("Initializing book in {:?}", path);
            // TODO: Implement init
            Ok(())
        }
        Commands::Build { path, output } => {
            println!("Building book from {:?} to {:?}", path, output);
            builder::build(&path, &output)
        }
        Commands::Serve { path, port } => {
            serve_book(&path, port)
        }
    }
}

fn serve_book(source: &PathBuf, port: u16) -> Result<()> {
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
                let dominated = event.paths.iter().any(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| matches!(e, "md" | "json" | "css" | "js" | "html"))
                        .unwrap_or(false)
                });
                if dominated {
                    println!("\n🔄 File changed, rebuilding...");
                    if let Err(e) = builder::build(&source_for_watcher, &temp_dir_for_watcher) {
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

    println!("\n📚 Serving book at http://localhost:{}/", port);
    println!("   🔥 Hot reload enabled - changes will auto-refresh");
    println!("   Press Ctrl+C to stop\n");

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
