//! `WebSearchProvider` — abstracción de búsqueda web.

use crate::error::{WebError, WebResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    #[serde(default)]
    pub score: f64,
}

impl SearchResult {
    pub fn new(
        title: impl Into<String>,
        url: impl Into<String>,
        snippet: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
            snippet: snippet.into(),
            score: 0.0,
        }
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub provider: String,
    pub searched_at: DateTime<Utc>,
}

impl SearchResponse {
    pub fn new(query: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            results: Vec::new(),
            provider: provider.into(),
            searched_at: Utc::now(),
        }
    }

    pub fn with_results(mut self, results: Vec<SearchResult>) -> Self {
        self.results = results;
        self
    }

    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn filter(&self, keyword: &str) -> Vec<&SearchResult> {
        let lower = keyword.to_lowercase();
        self.results
            .iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&lower) || r.snippet.to_lowercase().contains(&lower)
            })
            .collect()
    }

    pub fn top(&self, n: usize) -> Vec<&SearchResult> {
        let mut sorted: Vec<&SearchResult> = self.results.iter().collect();
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().take(n).collect()
    }

    pub fn summary(&self) -> String {
        format!(
            "Search '{}' via {}: {} results",
            self.query,
            self.provider,
            self.results.len()
        )
    }
}

pub trait WebSearchProvider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> WebResult<SearchResponse>;
}

#[derive(Debug, Default)]
pub struct MockSearchProvider {
    responses: std::collections::BTreeMap<String, Vec<SearchResult>>,
    calls: std::sync::Mutex<Vec<(String, usize)>>,
}

impl MockSearchProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_results(mut self, query: &str, results: Vec<SearchResult>) -> Self {
        self.responses.insert(query.to_string(), results);
        self
    }

    pub fn calls(&self) -> Vec<(String, usize)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl WebSearchProvider for MockSearchProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn search(&self, query: &str, limit: usize) -> WebResult<SearchResponse> {
        if query.trim().is_empty() {
            return Err(WebError::EmptyQuery);
        }

        self.calls.lock().unwrap().push((query.to_string(), limit));

        let results = self.responses.get(query).cloned().unwrap_or_default();
        let limited: Vec<SearchResult> = results.into_iter().take(limit).collect();

        Ok(SearchResponse::new(query, "mock").with_results(limited))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_result_new() {
        let r = SearchResult::new("Title", "https://x.com", "Snippet");
        assert_eq!(r.title, "Title");
        assert_eq!(r.url, "https://x.com");
        assert_eq!(r.score, 0.0);
    }

    #[test]
    fn search_result_with_score() {
        let r = SearchResult::new("T", "U", "S").with_score(0.95);
        assert_eq!(r.score, 0.95);
    }

    #[test]
    fn response_new_is_empty() {
        let resp = SearchResponse::new("query", "test");
        assert_eq!(resp.result_count(), 0);
        assert!(resp.is_empty());
    }

    #[test]
    fn response_with_results() {
        let resp = SearchResponse::new("query", "test").with_results(vec![
            SearchResult::new("A", "u1", "s1"),
            SearchResult::new("B", "u2", "s2"),
        ]);
        assert_eq!(resp.result_count(), 2);
    }

    #[test]
    fn response_filter_by_title() {
        let resp = SearchResponse::new("query", "test").with_results(vec![
            SearchResult::new("Rust programming", "u1", "s1"),
            SearchResult::new("Python programming", "u2", "s2"),
        ]);
        let rust = resp.filter("rust");
        assert_eq!(rust.len(), 1);
        assert_eq!(rust[0].title, "Rust programming");
    }

    #[test]
    fn response_filter_by_snippet() {
        let resp = SearchResponse::new("query", "test").with_results(vec![
            SearchResult::new("A", "u1", "mentions rust"),
            SearchResult::new("B", "u2", "mentions python"),
        ]);
        let rust = resp.filter("rust");
        assert_eq!(rust.len(), 1);
    }

    #[test]
    fn response_top_by_score() {
        let resp = SearchResponse::new("q", "test").with_results(vec![
            SearchResult::new("Low", "u1", "s").with_score(0.1),
            SearchResult::new("High", "u2", "s").with_score(0.9),
            SearchResult::new("Mid", "u3", "s").with_score(0.5),
        ]);
        let top2 = resp.top(2);
        assert_eq!(top2[0].title, "High");
        assert_eq!(top2[1].title, "Mid");
    }

    #[test]
    fn response_summary() {
        let resp = SearchResponse::new("rust", "test")
            .with_results(vec![SearchResult::new("A", "u1", "s")]);
        let s = resp.summary();
        assert!(s.contains("rust"));
        assert!(s.contains("1 results"));
    }

    #[test]
    fn mock_provider_returns_registered() {
        let provider = MockSearchProvider::new().with_results(
            "rust",
            vec![SearchResult::new(
                "Rust Lang",
                "https://rust-lang.org",
                "The language",
            )],
        );

        let resp = provider.search("rust", 10).unwrap();
        assert_eq!(resp.result_count(), 1);
        assert_eq!(resp.results[0].title, "Rust Lang");
    }

    #[test]
    fn mock_provider_empty_query_fails() {
        let provider = MockSearchProvider::new();
        let err = provider.search("", 10).unwrap_err();
        assert!(matches!(err, WebError::EmptyQuery));
    }

    #[test]
    fn mock_provider_unknown_query_returns_empty() {
        let provider = MockSearchProvider::new();
        let resp = provider.search("unknown", 10).unwrap();
        assert!(resp.is_empty());
    }

    #[test]
    fn mock_provider_respects_limit() {
        let provider = MockSearchProvider::new().with_results(
            "rust",
            vec![
                SearchResult::new("A", "u1", "s"),
                SearchResult::new("B", "u2", "s"),
                SearchResult::new("C", "u3", "s"),
            ],
        );
        let resp = provider.search("rust", 2).unwrap();
        assert_eq!(resp.result_count(), 2);
    }

    #[test]
    fn mock_provider_records_calls() {
        let provider = MockSearchProvider::new();
        provider.search("q1", 5).unwrap();
        provider.search("q2", 10).unwrap();
        assert_eq!(provider.call_count(), 2);
        assert_eq!(provider.calls()[0], ("q1".to_string(), 5));
    }

    #[test]
    fn mock_provider_name() {
        let provider = MockSearchProvider::new();
        assert_eq!(provider.name(), "mock");
    }

    #[test]
    fn response_serializes() {
        let resp = SearchResponse::new("q", "test")
            .with_results(vec![SearchResult::new("A", "u", "s").with_score(0.5)]);
        let json = serde_json::to_string(&resp).unwrap();
        let back: SearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.result_count(), 1);
    }
}
