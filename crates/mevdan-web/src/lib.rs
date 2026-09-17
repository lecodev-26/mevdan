//! # mevdan-web
//!
//! Browser y Web Search para MEVDAN.
//!
//! ## Concepto
//!
//! - **Browser** — obtener y entender contenido web (`WebPage`).
//! - **Web Search** — consultar proveedores y obtener resultados.
//!
//! ## Arquitectura
//!
//! El crate define dos **traits**:
//!
//! - `WebFetcher` — cómo obtener una página desde una URL.
//! - `WebSearchProvider` — cómo buscar.
//!
//! Las implementaciones reales (reqwest, DuckDuckGo, Brave, ...) se
//! enganchan en V5.9. V5.6 incluye implementaciones `Mock*` para tests.
//!
//! ## Estado del proyecto
//!
//! - **V5.6** ✅ — `WebUrl`, `WebPage`, `WebFetcher`, `WebSearchProvider`, `WebEngine`.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_web::WebUrl;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let url: WebUrl = "https://example.com/page?q=test".parse()?;
//! println!("Host: {}", url.host);
//! println!("Path: {}", url.normalized_path());
//! # Ok(())
//! # }
//! ```

pub mod engine;
pub mod error;
pub mod fetcher;
pub mod page;
pub mod search;
pub mod url;

// Re-exports de conveniencia.
pub use engine::WebEngine;
pub use error::{WebError, WebResult};
pub use fetcher::{MockFetcher, WebFetcher};
pub use page::{WebLink, WebMetadata, WebPage};
pub use search::{MockSearchProvider, SearchResponse, SearchResult, WebSearchProvider};
pub use url::{Scheme, WebUrl};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_search_then_fetch() {
        let provider = MockSearchProvider::new().with_results(
            "rust async",
            vec![
                SearchResult::new(
                    "Async in Rust",
                    "https://rust-lang.org/async",
                    "Guide to async Rust",
                )
                .with_score(0.95),
                SearchResult::new("Tokio", "https://tokio.rs", "Async runtime").with_score(0.80),
            ],
        );

        let u1: WebUrl = "https://rust-lang.org/async".parse().unwrap();
        let u2: WebUrl = "https://tokio.rs".parse().unwrap();

        let fetcher = MockFetcher::new()
            .with_response(
                &u1.to_string(),
                WebPage::new(u1.clone(), "Rust async guide content", 200).with_title("Async Rust"),
            )
            .with_response(
                &u2.to_string(),
                WebPage::new(u2.clone(), "Tokio documentation", 200).with_title("Tokio"),
            );

        let engine = WebEngine::new()
            .with_search(Box::new(provider))
            .with_fetcher(Box::new(fetcher));

        let response = engine.search("rust async", 10).unwrap();
        assert_eq!(response.result_count(), 2);
        assert_eq!(response.top(1)[0].title, "Async in Rust");

        let pages = engine.search_and_fetch("rust async", 10).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].1.title.as_deref(), Some("Async Rust"));
        assert_eq!(pages[1].1.title.as_deref(), Some("Tokio"));
    }

    #[test]
    fn full_flow_url_parsing() {
        let cases = [
            "https://example.com",
            "https://example.com/",
            "http://localhost:8080/api/v1?key=value",
            "https://api.github.com:443/repos/rust-lang/rust#readme",
        ];

        for case in &cases {
            let url: WebUrl = case.parse().expect(case);
            let roundtrip = url.to_string();
            assert_eq!(&roundtrip, case, "roundtrip failed for {}", case);
        }
    }
}
