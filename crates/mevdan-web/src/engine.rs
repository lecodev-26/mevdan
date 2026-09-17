//! `WebEngine` — fachada que combina fetcher y search.

use crate::{
    error::{WebError, WebResult},
    fetcher::WebFetcher,
    page::WebPage,
    search::{SearchResponse, WebSearchProvider},
    url::WebUrl,
};

pub struct WebEngine {
    fetcher: Option<Box<dyn WebFetcher>>,
    search_provider: Option<Box<dyn WebSearchProvider>>,
}

impl std::fmt::Debug for WebEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebEngine")
            .field("has_fetcher", &self.fetcher.is_some())
            .field("has_search", &self.search_provider.is_some())
            .finish()
    }
}

impl Default for WebEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WebEngine {
    pub fn new() -> Self {
        Self {
            fetcher: None,
            search_provider: None,
        }
    }

    pub fn with_fetcher(mut self, fetcher: Box<dyn WebFetcher>) -> Self {
        self.fetcher = Some(fetcher);
        self
    }

    pub fn with_search(mut self, provider: Box<dyn WebSearchProvider>) -> Self {
        self.search_provider = Some(provider);
        self
    }

    pub fn has_fetcher(&self) -> bool {
        self.fetcher.is_some()
    }

    pub fn has_search(&self) -> bool {
        self.search_provider.is_some()
    }

    pub fn fetch(&self, url: &WebUrl) -> WebResult<WebPage> {
        let fetcher = self.fetcher.as_ref().ok_or(WebError::NoFetcher)?;
        fetcher.fetch(url)
    }

    pub fn search(&self, query: &str, limit: usize) -> WebResult<SearchResponse> {
        let provider = self
            .search_provider
            .as_ref()
            .ok_or(WebError::NoSearchProvider)?;
        provider.search(query, limit)
    }

    pub fn search_and_fetch(
        &self,
        query: &str,
        limit: usize,
    ) -> WebResult<Vec<(crate::search::SearchResult, WebPage)>> {
        let response = self.search(query, limit)?;
        let mut out = Vec::new();

        for result in response.results {
            let url: WebUrl = match result.url.parse() {
                Ok(u) => u,
                Err(_) => continue,
            };
            match self.fetch(&url) {
                Ok(page) => out.push((result, page)),
                Err(_) => continue,
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fetcher::MockFetcher,
        search::{MockSearchProvider, SearchResult},
    };

    fn sample_url() -> WebUrl {
        "https://example.com".parse().unwrap()
    }

    #[test]
    fn engine_new_has_nothing() {
        let engine = WebEngine::new();
        assert!(!engine.has_fetcher());
        assert!(!engine.has_search());
    }

    #[test]
    fn engine_with_fetcher() {
        let fetcher = MockFetcher::new();
        let engine = WebEngine::new().with_fetcher(Box::new(fetcher));
        assert!(engine.has_fetcher());
    }

    #[test]
    fn engine_with_search() {
        let provider = MockSearchProvider::new();
        let engine = WebEngine::new().with_search(Box::new(provider));
        assert!(engine.has_search());
    }

    #[test]
    fn fetch_without_fetcher_fails() {
        let engine = WebEngine::new();
        let err = engine.fetch(&sample_url()).unwrap_err();
        assert!(matches!(err, WebError::NoFetcher));
    }

    #[test]
    fn search_without_provider_fails() {
        let engine = WebEngine::new();
        let err = engine.search("q", 10).unwrap_err();
        assert!(matches!(err, WebError::NoSearchProvider));
    }

    #[test]
    fn fetch_with_fetcher() {
        let url = sample_url();
        let page = WebPage::new(url.clone(), "Hello", 200);
        let fetcher = MockFetcher::new().with_response(&url.to_string(), page);

        let engine = WebEngine::new().with_fetcher(Box::new(fetcher));
        let result = engine.fetch(&url).unwrap();
        assert_eq!(result.text, "Hello");
    }

    #[test]
    fn search_with_provider() {
        let provider = MockSearchProvider::new().with_results(
            "rust",
            vec![SearchResult::new("Rust", "https://rust-lang.org", "lang")],
        );
        let engine = WebEngine::new().with_search(Box::new(provider));
        let resp = engine.search("rust", 10).unwrap();
        assert_eq!(resp.result_count(), 1);
    }

    #[test]
    fn search_and_fetch_combines() {
        let u1: WebUrl = "https://a.example.com".parse().unwrap();
        let u2: WebUrl = "https://b.example.com".parse().unwrap();

        let provider = MockSearchProvider::new().with_results(
            "query",
            vec![
                SearchResult::new("A", u1.to_string(), "s1"),
                SearchResult::new("B", u2.to_string(), "s2"),
            ],
        );

        let fetcher = MockFetcher::new()
            .with_response(&u1.to_string(), WebPage::new(u1.clone(), "Page A", 200))
            .with_response(&u2.to_string(), WebPage::new(u2.clone(), "Page B", 200));

        let engine = WebEngine::new()
            .with_search(Box::new(provider))
            .with_fetcher(Box::new(fetcher));

        let results = engine.search_and_fetch("query", 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.title, "A");
        assert_eq!(results[0].1.text, "Page A");
    }

    #[test]
    fn search_and_fetch_skips_failed_fetches() {
        let u1: WebUrl = "https://a.example.com".parse().unwrap();
        let u2: WebUrl = "https://b.example.com".parse().unwrap();

        let provider = MockSearchProvider::new().with_results(
            "q",
            vec![
                SearchResult::new("A", u1.to_string(), "s"),
                SearchResult::new("B", u2.to_string(), "s"),
            ],
        );

        let fetcher = MockFetcher::new()
            .with_response(&u1.to_string(), WebPage::new(u1.clone(), "Page A", 200));

        let engine = WebEngine::new()
            .with_search(Box::new(provider))
            .with_fetcher(Box::new(fetcher));

        let results = engine.search_and_fetch("q", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_and_fetch_skips_invalid_urls() {
        let provider = MockSearchProvider::new().with_results(
            "q",
            vec![
                SearchResult::new("A", "not a url", "s"),
                SearchResult::new("B", "https://valid.com", "s"),
            ],
        );

        let u2: WebUrl = "https://valid.com".parse().unwrap();
        let fetcher =
            MockFetcher::new().with_response(&u2.to_string(), WebPage::new(u2.clone(), "V", 200));

        let engine = WebEngine::new()
            .with_search(Box::new(provider))
            .with_fetcher(Box::new(fetcher));

        let results = engine.search_and_fetch("q", 10).unwrap();
        assert_eq!(results.len(), 1);
    }
}
