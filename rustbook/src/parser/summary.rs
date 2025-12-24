use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Summary {
    pub title: Option<String>,
    pub items: Vec<SummaryItem>,
}

#[derive(Debug, Clone)]
pub enum SummaryItem {
    /// A chapter with a link
    Link {
        title: String,
        path: Option<String>,
        children: Vec<SummaryItem>,
    },
    /// A separator (horizontal rule)
    Separator,
    /// A part header (unlinked heading)
    PartTitle(String),
}

impl Summary {
    pub fn parse(book_dir: &Path) -> Result<Self> {
        let summary_path = book_dir.join("SUMMARY.md");
        let content = fs::read_to_string(&summary_path)?;
        parse_summary(&content)
    }
}

/// Parse SUMMARY.md content into a Summary structure
pub fn parse_summary(content: &str) -> Result<Summary> {
    let mut title = None;
    let mut stack: Vec<(usize, Vec<SummaryItem>)> = vec![(0, Vec::new())];

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Check for title (# Summary or # Title)
        if trimmed.starts_with("# ") {
            title = Some(trimmed[2..].trim().to_string());
            continue;
        }

        // Check for separator (---)
        if trimmed == "---" || trimmed.chars().all(|c| c == '-') && trimmed.len() >= 3 {
            push_item(&mut stack, SummaryItem::Separator);
            continue;
        }

        // Check for part title (## Part Name or ### Part Name)
        if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            let part_title = trimmed.trim_start_matches('#').trim().to_string();
            push_item(&mut stack, SummaryItem::PartTitle(part_title));
            continue;
        }

        // Check for list item (* [Title](path) or - [Title](path))
        if let Some(item) = parse_list_item(line) {
            let indent = calculate_indent(line);

            // Pop stack until we find a parent with less indent
            // Use > (not >=) because stored level is indent+1, and items at that level should stay
            while stack.len() > 1 && stack.last().map(|(i, _)| *i > indent).unwrap_or(false) {
                let (_, children) = stack.pop().unwrap();
                if let Some((_, parent_items)) = stack.last_mut() {
                    if let Some(SummaryItem::Link { children: ref mut c, .. }) = parent_items.last_mut() {
                        *c = children;
                    }
                }
            }

            // Push the new item
            push_item(&mut stack, item);

            // Start a new level for potential children
            stack.push((indent + 1, Vec::new()));
        }
    }

    // Collapse the stack
    while stack.len() > 1 {
        let (_, children) = stack.pop().unwrap();
        if let Some((_, parent_items)) = stack.last_mut() {
            if let Some(SummaryItem::Link { children: ref mut c, .. }) = parent_items.last_mut() {
                *c = children;
            }
        }
    }

    let items = stack.pop().unwrap().1;

    Ok(Summary { title, items })
}

fn calculate_indent(line: &str) -> usize {
    let mut spaces = 0;
    let mut tabs = 0;
    for ch in line.chars() {
        match ch {
            ' ' => spaces += 1,
            '\t' => tabs += 1,
            _ => break,
        }
    }
    // Tab = 1 level, 4 spaces = 1 level (matching HonKit behavior)
    tabs + (spaces / 4)
}

fn push_item(stack: &mut Vec<(usize, Vec<SummaryItem>)>, item: SummaryItem) {
    if let Some((_, items)) = stack.last_mut() {
        items.push(item);
    }
}

fn parse_list_item(line: &str) -> Option<SummaryItem> {
    let trimmed = line.trim();

    // Must start with * or -
    if !trimmed.starts_with('*') && !trimmed.starts_with('-') {
        return None;
    }

    let rest = trimmed[1..].trim();

    // Check for linked item: [Title](path)
    if rest.starts_with('[') {
        if let Some(title_end) = rest.find(']') {
            let title = rest[1..title_end].to_string();

            // Look for the path
            let after_title = &rest[title_end + 1..];
            if after_title.starts_with('(') {
                if let Some(path_end) = after_title.find(')') {
                    let path = after_title[1..path_end].to_string();
                    let path = if path.is_empty() || path == "#" {
                        None
                    } else {
                        // Normalize path: remove leading ./ if present
                        let normalized = path.trim_start_matches("./").to_string();
                        Some(normalized)
                    };
                    return Some(SummaryItem::Link {
                        title,
                        path,
                        children: Vec::new(),
                    });
                }
            }
        }
    }

    // Plain text item (no link)
    if !rest.is_empty() && !rest.starts_with('[') {
        return Some(SummaryItem::Link {
            title: rest.to_string(),
            path: None,
            children: Vec::new(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_summary() {
        let content = r#"# Summary

* [Introduction](README.md)
* [Chapter 1](chapter1.md)
    * [Section 1.1](chapter1/section1.md)
    * [Section 1.2](chapter1/section2.md)
* [Chapter 2](chapter2.md)
"#;

        let summary = parse_summary(content).unwrap();
        assert_eq!(summary.title, Some("Summary".to_string()));
        assert_eq!(summary.items.len(), 3);
    }

    #[test]
    fn test_parse_nested_summary() {
        let content = r#"# Summary

* [表紙](README.md)
* 顧客画面
    * ポートフォリオ
        * [TOP](Customer/AssetStatus/PortfolioTop.md)
        * [国内株式現物](./Customer/AssetStatus/PortfolioStock.md)
"#;

        let summary = parse_summary(content).unwrap();
        assert_eq!(summary.items.len(), 2);

        // Verify children structure
        if let SummaryItem::Link { title, children, .. } = &summary.items[1] {
            assert_eq!(title, "顧客画面");
            assert_eq!(children.len(), 1, "顧客画面 should have 1 child (ポートフォリオ)");

            if let SummaryItem::Link { title: child_title, children: grandchildren, .. } = &children[0] {
                assert_eq!(child_title, "ポートフォリオ");
                assert_eq!(grandchildren.len(), 2, "ポートフォリオ should have 2 children (TOP, 国内株式現物)");
            } else {
                panic!("Expected Link for ポートフォリオ");
            }
        } else {
            panic!("Expected Link for 顧客画面");
        }
    }
}
