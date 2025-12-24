use crate::parser::{BookConfig, Summary, SummaryItem};
use anyhow::Result;
use tera::{Context, Tera};

pub struct Templates {
    tera: Tera,
}

impl Templates {
    pub fn new(_config: &BookConfig) -> Result<Self> {
        let mut tera = Tera::default();

        // Register the main page template
        tera.add_raw_template("page.html", PAGE_TEMPLATE)?;

        Ok(Self { tera })
    }

    pub fn render_page(
        &self,
        title: &str,
        content: &str,
        root_path: &str,
        config: &BookConfig,
        summary: &Summary,
        current_path: Option<&str>,
    ) -> Result<String> {
        let mut context = Context::new();

        context.insert("title", title);
        context.insert("book_title", &config.title);
        context.insert("content", content);
        context.insert("root_path", root_path);

        // Generate sidebar HTML - links are relative to base href
        let sidebar = generate_sidebar(&summary.items, current_path, "");
        context.insert("sidebar", &sidebar);

        // Check plugin features
        context.insert("back_to_top", &config.is_plugin_enabled("back-to-top-button"));
        context.insert("collapsible", &config.is_plugin_enabled("collapsible-chapters"));
        context.insert("mermaid", &config.is_plugin_enabled("mermaid-md-adoc"));

        // Custom styles
        let has_custom_style = config.get_website_style().is_some();
        context.insert("has_custom_style", &has_custom_style);

        let html = self.tera.render("page.html", &context)?;
        Ok(html)
    }
}

fn generate_sidebar(items: &[SummaryItem], current_path: Option<&str>, prefix: &str) -> String {
    let mut html = String::new();

    eprintln!("DEBUG generate_sidebar: processing {} items", items.len());

    for item in items {
        match item {
            SummaryItem::Link { title, path, children } => {
                let html_path = path.as_ref().map(|p| p.replace(".md", ".html"));
                let is_active = current_path.map(|cp| {
                    html_path.as_ref().map(|hp| cp == hp).unwrap_or(false)
                }).unwrap_or(false);

                let has_children = !children.is_empty();
                eprintln!("DEBUG: {} has_children={} (len={})", title, has_children, children.len());
                let active_class = if is_active { " active" } else { "" };
                let expandable_class = if has_children { " expandable" } else { "" };

                html.push_str(&format!(
                    r#"<li class="chapter{}{}">"#,
                    active_class, expandable_class
                ));

                if let Some(ref hp) = html_path {
                    html.push_str(&format!(
                        r#"<a href="{}{}">{}</a>"#,
                        prefix, hp, html_escape(title)
                    ));
                } else {
                    html.push_str(&format!(
                        r#"<span class="chapter-title">{}</span>"#,
                        html_escape(title)
                    ));
                }

                if has_children {
                    html.push_str("<ul class=\"articles\">");
                    html.push_str(&generate_sidebar(children, current_path, prefix));
                    html.push_str("</ul>");
                }

                html.push_str("</li>");
            }
            SummaryItem::Separator => {
                html.push_str(r#"<li class="divider"></li>"#);
            }
            SummaryItem::PartTitle(part_title) => {
                html.push_str(&format!(
                    r#"<li class="part-title"><span>{}</span></li>"#,
                    html_escape(part_title)
                ));
            }
        }
    }

    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const PAGE_TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <base href="{{ root_path }}">
    <title>{{ title }} | {{ book_title }}</title>
    <link rel="stylesheet" href="gitbook/gitbook.css">
    {% if has_custom_style %}
    <link rel="stylesheet" href="gitbook/style.css">
    {% endif %}
    {% if mermaid %}
    <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
    <script>mermaid.initialize({startOnLoad:true});</script>
    {% endif %}
</head>
<body class="book font-family-1">
    <div class="book-summary">
        <nav role="navigation">
            <ul class="summary">
                {{ sidebar | safe }}
            </ul>
        </nav>
    </div>

    <div class="book-body">
        <div class="sidebar-toggle" title="Toggle Sidebar">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="3" y1="6" x2="21" y2="6"></line>
                <line x1="3" y1="12" x2="21" y2="12"></line>
                <line x1="3" y1="18" x2="21" y2="18"></line>
            </svg>
        </div>
        <div class="body-inner">
            <div class="page-wrapper">
                <div class="page-inner">
                    <section class="markdown-section">
                        {{ content | safe }}
                    </section>
                </div>
            </div>
        </div>
    </div>

    {% if back_to_top %}
    <a href="#" class="back-to-top" title="Back to top">
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 15l-6-6-6 6"/>
        </svg>
    </a>
    {% endif %}

    <script src="gitbook/gitbook.js"></script>
    {% if collapsible %}
    <script src="gitbook/collapsible.js"></script>
    {% endif %}
</body>
</html>
"##;
