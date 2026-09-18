# MEVDAN — Web Specification

`mevdan-web` covers two capabilities:

- **Browser** — fetching and understanding web pages.
- **Web Search** — querying search providers.

Both share the "get content from the web" abstraction.

## Concepts

### WebUrl

HTTP/HTTPS URL with a basic parser.

```rust
let url: WebUrl = "https://api.example.com:8443/v1/search?q=test#top".parse()?;

assert_eq!(url.scheme, Scheme::Https);
assert_eq!(url.host, "api.example.com");
assert_eq!(url.port, Some(8443));
assert_eq!(url.path, "/v1/search");
assert_eq!(url.query.as_deref(), Some("q=test"));
assert_eq!(url.fragment.as_deref(), Some("top"));
Helpers:

effective_port() — explicit port or scheme default (80/443).

authority() — host:port.

normalized_path() — ensures leading /.

request_target() — path + query for HTTP requests.

Parser is basic: no percent-encoding, no userinfo, no IPv6 hosts.
Sufficient for HTTP/HTTPS URLs.

WebLink
rust
pub struct WebLink {
    pub text: String,
    pub href: String,
}
WebMetadata
rust
pub struct WebMetadata {
    pub description: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub canonical_url: Option<String>,
}
WebPage
rust
pub struct WebPage {
    pub url: WebUrl,
    pub final_url: WebUrl,     // after redirects
    pub title: Option<String>,
    pub text: String,          // plain text
    pub links: Vec<WebLink>,
    pub metadata: WebMetadata,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub fetched_at: DateTime<Utc>,
}
Helpers:

is_success() (2xx), is_redirect() (3xx), is_client_error()
(4xx), is_server_error() (5xx).

preview(max_chars) — UTF-8 safe snippet.

summary() — one-liner.

WebFetcher
rust
pub trait WebFetcher: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn fetch(&self, url: &WebUrl) -> WebResult<WebPage>;
}
MockFetcher
Predefined responses for testing:

rust
let fetcher = MockFetcher::new()
    .with_response("https://example.com", WebPage::new(url, "content", 200));
Records calls for verification.

SearchResult
rust
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub score: f64,
}
SearchResponse
rust
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub provider: String,
    pub searched_at: DateTime<Utc>,
}
Helpers:

filter(keyword) — filter by title/snippet.

top(n) — top N by score.

summary().

WebSearchProvider
rust
pub trait WebSearchProvider: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> WebResult<SearchResponse>;
}
MockSearchProvider
Predefined results per query. Rejects empty queries.

WebEngine
Combines both:

rust
let engine = WebEngine::new()
    .with_search(Box::new(provider))
    .with_fetcher(Box::new(fetcher));

// Search.
let response = engine.search("rust async", 10)?;

// Fetch.
let page = engine.fetch(&url)?;

// Combined.
let pages = engine.search_and_fetch("rust async", 10)?;
search_and_fetch skips invalid URLs and fetch failures.

What this is NOT
No real HTTP. The WebFetcher trait defines the interface;
a reqwest-based implementation arrives in V5.9.

No HTML parsing. WebPage.text is filled by the fetcher.

No real providers. DuckDuckGo, Brave, Google — V5.9.

No browser automation. No clicks, no forms, no JS execution.

No caching.

No redirect handling. The fetcher is responsible.

Design notes
Interfaces first. The traits are stable; implementations
come later.

Mockable. Every trait has a mock implementation for testing.

No deps. Only std + serde + chrono.

UTF-8 safe. Preview never splits multibyte characters.
