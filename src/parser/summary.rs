use anyhow::Result;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
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
/// Uses pulldown-cmark to parse Markdown structure (like HonKit)
pub fn parse_summary(content: &str) -> Result<Summary> {
    let mut title = None;
    let mut items = Vec::new();
    let parser = Parser::new(content);

    // State tracking
    let mut in_list_stack: Vec<Vec<SummaryItem>> = Vec::new(); // Stack of list items at each depth
    let mut current_link: Option<(String, Option<String>)> = None; // (title, path)
    let mut current_text = String::new();
    let mut in_heading = false;
    let mut heading_level = 0;
    let mut pending_item_text = String::new(); // Text for plain items (no link)

    for event in parser {
        match event {
            // Heading (# Title, ## Part, ### Part)
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = level as usize;
                current_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let text = current_text.trim().to_string();
                if heading_level == 1 {
                    // # Title
                    title = Some(text);
                } else if heading_level == 2 || heading_level == 3 {
                    // ## Part or ### Part
                    items.push(SummaryItem::PartTitle(text));
                }
                current_text.clear();
            }

            // List start/end
            Event::Start(Tag::List(_)) => {
                // Before starting nested list, flush any pending link from parent item
                if !in_list_stack.is_empty() {
                    if let Some((link_title, link_path)) = current_link.take() {
                        if let Some(current_list) = in_list_stack.last_mut() {
                            current_list.push(SummaryItem::Link {
                                title: link_title,
                                path: link_path,
                                children: Vec::new(),
                            });
                        }
                    } else if !pending_item_text.is_empty() {
                        // Plain text item
                        if let Some(current_list) = in_list_stack.last_mut() {
                            current_list.push(SummaryItem::Link {
                                title: pending_item_text.trim().to_string(),
                                path: None,
                                children: Vec::new(),
                            });
                        }
                        pending_item_text.clear();
                    }
                }
                in_list_stack.push(Vec::new());
            }
            Event::End(TagEnd::List(_)) => {
                if let Some(completed_items) = in_list_stack.pop() {
                    if in_list_stack.is_empty() {
                        // Top-level list completed, add to items
                        items.extend(completed_items);
                    } else {
                        // Nested list completed, attach as children to last item in parent
                        if let Some(parent_list) = in_list_stack.last_mut() {
                            if let Some(SummaryItem::Link { children, .. }) = parent_list.last_mut() {
                                *children = completed_items;
                            }
                        }
                    }
                }
            }

            // List item
            Event::Start(Tag::Item) => {
                current_link = None;
                current_text.clear();
                pending_item_text.clear();
            }
            Event::End(TagEnd::Item) => {
                // Only add item here if it wasn't already added when nested list started
                if let Some(current_list) = in_list_stack.last_mut() {
                    if let Some((link_title, link_path)) = current_link.take() {
                        current_list.push(SummaryItem::Link {
                            title: link_title,
                            path: link_path,
                            children: Vec::new(),
                        });
                    } else if !pending_item_text.is_empty() {
                        // Plain text item (no link)
                        current_list.push(SummaryItem::Link {
                            title: pending_item_text.trim().to_string(),
                            path: None,
                            children: Vec::new(),
                        });
                    }
                }
                current_text.clear();
                pending_item_text.clear();
            }

            // Link
            Event::Start(Tag::Link { dest_url, .. }) => {
                current_text.clear();
                let path = dest_url.to_string();
                let path = if path.is_empty() || path == "#" {
                    None
                } else {
                    // Normalize path: remove leading ./ if present
                    Some(path.trim_start_matches("./").to_string())
                };
                current_link = Some((String::new(), path));
            }
            Event::End(TagEnd::Link) => {
                if let Some((ref mut link_title, _)) = current_link {
                    *link_title = current_text.trim().to_string();
                }
                current_text.clear();
            }

            // Horizontal rule (separator)
            Event::Rule => {
                items.push(SummaryItem::Separator);
            }

            // Text content
            Event::Text(text) => {
                if in_heading {
                    current_text.push_str(&text);
                } else if !in_list_stack.is_empty() {
                    current_text.push_str(&text);
                    // Also track for plain items (text outside links)
                    if current_link.is_none() {
                        pending_item_text.push_str(&text);
                    }
                }
            }
            Event::Code(code) => {
                if in_heading {
                    current_text.push_str(&code);
                } else if !in_list_stack.is_empty() {
                    current_text.push_str(&code);
                    if current_link.is_none() {
                        pending_item_text.push_str(&code);
                    }
                }
            }

            _ => {}
        }
    }

    Ok(Summary { title, items })
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

    #[test]
    fn test_parse_2space_indent() {
        // Test 2-space indentation (like kcsta-trade-bff)
        let content = r#"# Summary

* [Introduction](README.md)
* [Chapter 1](chapter1.md)
  * [Section 1.1](chapter1/section1.md)
  * [Section 1.2](chapter1/section2.md)
    * [Subsection 1.2.1](chapter1/section2/sub1.md)
* [Chapter 2](chapter2.md)
"#;

        let summary = parse_summary(content).unwrap();
        assert_eq!(summary.items.len(), 3, "Should have 3 top-level items");

        // Verify Chapter 1 has 2 children
        if let SummaryItem::Link { title, children, .. } = &summary.items[1] {
            assert_eq!(title, "Chapter 1");
            assert_eq!(children.len(), 2, "Chapter 1 should have 2 children");

            // Verify Section 1.2 has 1 child (Subsection 1.2.1)
            if let SummaryItem::Link { title: sec_title, children: sec_children, .. } = &children[1] {
                assert_eq!(sec_title, "Section 1.2");
                assert_eq!(sec_children.len(), 1, "Section 1.2 should have 1 child");
            } else {
                panic!("Expected Link for Section 1.2");
            }
        } else {
            panic!("Expected Link for Chapter 1");
        }
    }

    #[test]
    fn test_parse_4space_indent() {
        // Test 4-space indentation (traditional)
        let content = r#"# Summary

* [Introduction](README.md)
* [Chapter 1](chapter1.md)
    * [Section 1.1](chapter1/section1.md)
    * [Section 1.2](chapter1/section2.md)
        * [Subsection 1.2.1](chapter1/section2/sub1.md)
* [Chapter 2](chapter2.md)
"#;

        let summary = parse_summary(content).unwrap();
        assert_eq!(summary.items.len(), 3, "Should have 3 top-level items");

        // Verify Chapter 1 has 2 children
        if let SummaryItem::Link { title, children, .. } = &summary.items[1] {
            assert_eq!(title, "Chapter 1");
            assert_eq!(children.len(), 2, "Chapter 1 should have 2 children");

            // Verify Section 1.2 has 1 child (Subsection 1.2.1)
            if let SummaryItem::Link { title: sec_title, children: sec_children, .. } = &children[1] {
                assert_eq!(sec_title, "Section 1.2");
                assert_eq!(sec_children.len(), 1, "Section 1.2 should have 1 child");
            } else {
                panic!("Expected Link for Section 1.2");
            }
        } else {
            panic!("Expected Link for Chapter 1");
        }
    }

    #[test]
    fn test_parse_mixed_indent() {
        // Test mixed indentation (2 and 4 spaces in same file)
        // With pulldown-cmark, this should work correctly
        let content = r#"# Summary

* [Item 1](item1.md)
  * [Item 1.1](item1-1.md)
* [Item 2](item2.md)
    * [Item 2.1](item2-1.md)
"#;

        let summary = parse_summary(content).unwrap();
        assert_eq!(summary.items.len(), 2, "Should have 2 top-level items");

        // Verify Item 1 has 1 child
        if let SummaryItem::Link { title, children, .. } = &summary.items[0] {
            assert_eq!(title, "Item 1");
            assert_eq!(children.len(), 1, "Item 1 should have 1 child");
        } else {
            panic!("Expected Link for Item 1");
        }

        // Verify Item 2 has 1 child
        if let SummaryItem::Link { title, children, .. } = &summary.items[1] {
            assert_eq!(title, "Item 2");
            assert_eq!(children.len(), 1, "Item 2 should have 1 child");
        } else {
            panic!("Expected Link for Item 2");
        }
    }

    #[test]
    fn test_parse_tab_indent() {
        // Test tab indentation
        let content = "# Summary\n\n* [Item 1](item1.md)\n\t* [Item 1.1](item1-1.md)\n* [Item 2](item2.md)\n";

        let summary = parse_summary(content).unwrap();
        assert_eq!(summary.items.len(), 2, "Should have 2 top-level items");

        if let SummaryItem::Link { title, children, .. } = &summary.items[0] {
            assert_eq!(title, "Item 1");
            assert_eq!(children.len(), 1, "Item 1 should have 1 child (tab-indented)");
        } else {
            panic!("Expected Link for Item 1");
        }
    }
}
