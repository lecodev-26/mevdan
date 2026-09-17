//! `WebPage` — contenido de una página web.

use crate::url::WebUrl;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebLink {
    pub text: String,
    pub href: String,
}

impl WebLink {
    pub fn new(text: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            href: href.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WebMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPage {
    pub url: WebUrl,
    pub final_url: WebUrl,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub text: String,
    #[serde(default)]
    pub links: Vec<WebLink>,
    #[serde(default)]
    pub metadata: WebMetadata,
    pub status_code: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub fetched_at: DateTime<Utc>,
}

impl WebPage {
    pub fn new(url: WebUrl, text: impl Into<String>, status_code: u16) -> Self {
        Self {
            final_url: url.clone(),
            url,
            title: None,
            text: text.into(),
            links: Vec::new(),
            metadata: WebMetadata::default(),
            status_code,
            content_type: None,
            fetched_at: Utc::now(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_links(mut self, links: Vec<WebLink>) -> Self {
        self.links = links;
        self
    }

    pub fn with_metadata(mut self, metadata: WebMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_final_url(mut self, url: WebUrl) -> Self {
        self.final_url = url;
        self
    }

    pub fn with_content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());
        self
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status_code)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    pub fn preview(&self, max_chars: usize) -> String {
        if self.text.len() <= max_chars {
            self.text.clone()
        } else {
            let mut end = max_chars;
            while !self.text.is_char_boundary(end) && end > 0 {
                end -= 1;
            }
            format!("{}...", &self.text[..end])
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{} [{}] {} chars, {} links{}",
            self.title.as_deref().unwrap_or(&self.final_url.to_string()),
            self.status_code,
            self.text.len(),
            self.links.len(),
            if self.final_url != self.url {
                format!(" (redirected from {})", self.url)
            } else {
                String::new()
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_url() -> WebUrl {
        "https://example.com/page".parse().unwrap()
    }

    #[test]
    fn link_new() {
        let l = WebLink::new("Click", "/next");
        assert_eq!(l.text, "Click");
        assert_eq!(l.href, "/next");
    }

    #[test]
    fn metadata_default() {
        let m = WebMetadata::default();
        assert!(m.description.is_none());
        assert!(m.author.is_none());
    }

    #[test]
    fn page_new() {
        let page = WebPage::new(sample_url(), "Hello world", 200);
        assert_eq!(page.status_code, 200);
        assert_eq!(page.text, "Hello world");
        assert_eq!(page.url, page.final_url);
        assert!(page.is_success());
    }

    #[test]
    fn page_with_title() {
        let page = WebPage::new(sample_url(), "content", 200).with_title("My Page");
        assert_eq!(page.title.as_deref(), Some("My Page"));
    }

    #[test]
    fn page_with_links() {
        let page = WebPage::new(sample_url(), "content", 200)
            .with_links(vec![WebLink::new("A", "/a"), WebLink::new("B", "/b")]);
        assert_eq!(page.link_count(), 2);
    }

    #[test]
    fn page_status_checks() {
        let ok = WebPage::new(sample_url(), "", 200);
        assert!(ok.is_success());
        assert!(!ok.is_redirect());
        assert!(!ok.is_client_error());
        assert!(!ok.is_server_error());

        let redirect = WebPage::new(sample_url(), "", 301);
        assert!(redirect.is_redirect());

        let not_found = WebPage::new(sample_url(), "", 404);
        assert!(not_found.is_client_error());

        let error = WebPage::new(sample_url(), "", 500);
        assert!(error.is_server_error());
    }

    #[test]
    fn page_preview_short() {
        let page = WebPage::new(sample_url(), "short text", 200);
        assert_eq!(page.preview(100), "short text");
    }

    #[test]
    fn page_preview_long() {
        let page = WebPage::new(
            sample_url(),
            "this is a long text that will be truncated",
            200,
        );
        let p = page.preview(10);
        assert!(p.ends_with("..."));
    }

    #[test]
    fn page_preview_utf8_safe() {
        let page = WebPage::new(sample_url(), "café y más", 200);
        let p = page.preview(5);
        assert!(p.len() <= 8);
    }

    #[test]
    fn page_summary() {
        let page = WebPage::new(sample_url(), "content", 200)
            .with_title("Title")
            .with_links(vec![WebLink::new("x", "/x")]);
        let s = page.summary();
        assert!(s.contains("Title"));
        assert!(s.contains("200"));
        assert!(s.contains("1 links"));
    }

    #[test]
    fn page_summary_redirect() {
        let from: WebUrl = "https://example.com/old".parse().unwrap();
        let to: WebUrl = "https://example.com/new".parse().unwrap();
        let page = WebPage::new(from, "content", 200).with_final_url(to);
        let s = page.summary();
        assert!(s.contains("redirected from"));
    }

    #[test]
    fn page_serializes() {
        let page = WebPage::new(sample_url(), "content", 200).with_title("Title");
        let json = serde_json::to_string(&page).unwrap();
        let back: WebPage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, page.title);
        assert_eq!(back.status_code, page.status_code);
    }
}
