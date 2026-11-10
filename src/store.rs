//! Saved links on disk: one JSON file in `~/Library/Application Support/Links/links.json`.
//! `LINKS_FILE` overrides the path (tests and headless snapshots point it elsewhere).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub id: u64,
    pub url: String,
    pub title: String,
    /// Seconds since the Unix epoch.
    pub added: u64,
    /// None for links saved before categories, or left uncategorized.
    #[serde(default)]
    pub category: Option<Category>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Vm,
    Design,
    Content,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Vm, Category::Design, Category::Content];

    pub fn label(self) -> &'static str {
        match self {
            Category::Vm => "VM",
            Category::Design => "Design",
            Category::Content => "Content",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Category::Vm => "vm",
            Category::Design => "design",
            Category::Content => "content",
        }
    }

    /// The next one round the cycle none → VM → Design → Content → none, for retagging with one click.
    pub fn cycle(c: Option<Category>) -> Option<Category> {
        match c {
            None => Some(Category::Vm),
            Some(Category::Vm) => Some(Category::Design),
            Some(Category::Design) => Some(Category::Content),
            Some(Category::Content) => None,
        }
    }
}

impl Link {
    /// What the row shows as its name: the title, else the host.
    pub fn name(&self) -> &str {
        if self.title.trim().is_empty() { host(&self.url) } else { self.title.trim() }
    }
}

pub fn path() -> PathBuf {
    if let Some(p) = std::env::var_os("LINKS_FILE") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join("Library/Application Support/Links/links.json")
}

pub fn load() -> Vec<Link> {
    let Ok(bytes) = std::fs::read(path()) else { return Vec::new() };
    serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        eprintln!("links: couldn't read {}: {e}", path().display());
        Vec::new()
    })
}

/// Writes to a temporary file and renames it over the old one, so a crash mid-write never
/// leaves a half-written list.
pub fn save(links: &[Link]) {
    let path = path();
    let result = (|| -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(links)?)?;
        std::fs::rename(&tmp, &path)
    })();
    if let Err(e) = result {
        eprintln!("links: couldn't save {}: {e}", path.display());
    }
}

/// Turns what was typed into a URL to open: trims it and adds `https://` when there's no scheme.
/// Returns None for text that can't be a web address.
pub fn normalize(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() || s.contains(char::is_whitespace) {
        return None;
    }
    let url = if s.starts_with("http://") || s.starts_with("https://") { s.to_string() } else { format!("https://{s}") };
    let h = host(&url);
    (!h.is_empty() && (h.contains('.') || h == "localhost" || h.starts_with("localhost:"))).then_some(url)
}

/// `https://www.example.com/a?b` → `example.com`.
pub fn host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let h = rest.split(['/', '?', '#']).next().unwrap_or("");
    h.strip_prefix("www.").unwrap_or(h)
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_typed_addresses() {
        assert_eq!(normalize(" example.com ").as_deref(), Some("https://example.com"));
        assert_eq!(normalize("http://a.io/x").as_deref(), Some("http://a.io/x"));
        assert_eq!(normalize("localhost:3000").as_deref(), Some("https://localhost:3000"));
        assert_eq!(normalize("not a url"), None);
        assert_eq!(normalize("word"), None);
        assert_eq!(normalize(""), None);
    }

    #[test]
    fn links_saved_before_categories_still_load() {
        let old = r#"[{"id":1,"url":"https://a.io","title":"","added":5}]"#;
        let links: Vec<Link> = serde_json::from_str(old).unwrap();
        assert_eq!(links[0].category, None);
    }

    #[test]
    fn host_strips_scheme_www_and_path() {
        assert_eq!(host("https://www.example.com/a?b#c"), "example.com");
        assert_eq!(host("http://news.ycombinator.com"), "news.ycombinator.com");
    }
}
