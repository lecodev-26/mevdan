//! `WebFetcher` — abstracción de un cliente HTTP.

use crate::{
    error::{WebError, WebResult},
    page::WebPage,
    url::WebUrl,
};

pub trait WebFetcher: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn fetch(&self, url: &WebUrl) -> WebResult<WebPage>;
}

#[derive(Debug, Default)]
pub struct MockFetcher {
    responses: std::collections::BTreeMap<String, WebPage>,
    calls: std::sync::Mutex<Vec<String>>,
}

impl MockFetcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_response(mut self, url: &str, page: WebPage) -> Self {
        self.responses.insert(url.to_string(), page);
        self
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl WebFetcher for MockFetcher {
    fn name(&self) -> &str {
        "mock"
    }

    fn fetch(&self, url: &WebUrl) -> WebResult<WebPage> {
        self.calls.lock().unwrap().push(url.to_string());

        if let Some(page) = self.responses.get(&url.to_string()) {
            return Ok(page.clone());
        }

        Err(WebError::FetchFailed(format!(
            "no mock response for {}",
            url
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_url() -> WebUrl {
        "https://example.com".parse().unwrap()
    }

    fn sample_page(url: WebUrl) -> WebPage {
        WebPage::new(url, "Hello world", 200)
    }

    #[test]
    fn mock_fetcher_returns_registered_response() {
        let url = sample_url();
        let fetcher = MockFetcher::new().with_response(&url.to_string(), sample_page(url.clone()));

        let page = fetcher.fetch(&url).unwrap();
        assert_eq!(page.text, "Hello world");
        assert_eq!(page.status_code, 200);
    }

    #[test]
    fn mock_fetcher_fails_when_no_response() {
        let fetcher = MockFetcher::new();
        let url = sample_url();
        let err = fetcher.fetch(&url).unwrap_err();
        assert!(matches!(err, WebError::FetchFailed(_)));
    }

    #[test]
    fn mock_fetcher_records_calls() {
        let url = sample_url();
        let fetcher = MockFetcher::new().with_response(&url.to_string(), sample_page(url.clone()));

        let _ = fetcher.fetch(&url);
        let _ = fetcher.fetch(&url);

        assert_eq!(fetcher.call_count(), 2);
        assert_eq!(fetcher.calls()[0], url.to_string());
    }

    #[test]
    fn mock_fetcher_name() {
        let fetcher = MockFetcher::new();
        assert_eq!(fetcher.name(), "mock");
    }

    #[test]
    fn mock_fetcher_multiple_urls() {
        let u1: WebUrl = "https://a.example.com".parse().unwrap();
        let u2: WebUrl = "https://b.example.com".parse().unwrap();

        let fetcher = MockFetcher::new()
            .with_response(&u1.to_string(), WebPage::new(u1.clone(), "A", 200))
            .with_response(&u2.to_string(), WebPage::new(u2.clone(), "B", 200));

        assert_eq!(fetcher.fetch(&u1).unwrap().text, "A");
        assert_eq!(fetcher.fetch(&u2).unwrap().text, "B");
    }
}
