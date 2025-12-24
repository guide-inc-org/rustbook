use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Language {
    pub code: String,
    pub title: String,
}

/// Parse LANGS.md to get available languages
/// Format:
/// * [Japanese](jp/)
/// * [Vietnamese](vn/)
pub fn parse_langs(book_dir: &Path) -> Result<Vec<Language>> {
    let langs_path = book_dir.join("LANGS.md");

    if !langs_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&langs_path)?;
    let mut languages = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Match pattern: * [Title](code/) or - [Title](code/)
        if (line.starts_with('*') || line.starts_with('-')) && line.contains('[') && line.contains("](") {
            if let Some(lang) = parse_lang_line(line) {
                languages.push(lang);
            }
        }
    }

    Ok(languages)
}

fn parse_lang_line(line: &str) -> Option<Language> {
    // Extract title between [ and ]
    let title_start = line.find('[')? + 1;
    let title_end = line.find(']')?;
    let title = line[title_start..title_end].to_string();

    // Extract code between ]( and )
    let code_start = line.find("](")? + 2;
    let code_end = line.rfind(')')?;
    let mut code = line[code_start..code_end].to_string();

    // Remove trailing slash if present
    if code.ends_with('/') {
        code.pop();
    }

    Some(Language { code, title })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lang_line() {
        let lang = parse_lang_line("* [Japanese](jp/)").unwrap();
        assert_eq!(lang.code, "jp");
        assert_eq!(lang.title, "Japanese");

        let lang = parse_lang_line("- [Vietnamese](vn/)").unwrap();
        assert_eq!(lang.code, "vn");
        assert_eq!(lang.title, "Vietnamese");
    }
}
