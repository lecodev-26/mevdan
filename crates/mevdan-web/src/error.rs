//! Errores del crate `mevdan-web`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("fetch failed: {0}")]
    FetchFailed(String),

    #[error("search failed: {0}")]
    SearchFailed(String),

    #[error("no fetcher configured")]
    NoFetcher,

    #[error("no search provider configured")]
    NoSearchProvider,

    #[error("unsupported scheme: {0}")]
    UnsupportedScheme(String),

    #[error("empty query")]
    EmptyQuery,

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type WebResult<T> = Result<T, WebError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_url_displays() {
        let err = WebError::InvalidUrl("not a url".into());
        assert_eq!(err.to_string(), "invalid URL: not a url");
    }

    #[test]
    fn fetch_failed_displays() {
        let err = WebError::FetchFailed("timeout".into());
        assert_eq!(err.to_string(), "fetch failed: timeout");
    }

    #[test]
    fn search_failed_displays() {
        let err = WebError::SearchFailed("quota exceeded".into());
        assert_eq!(err.to_string(), "search failed: quota exceeded");
    }

    #[test]
    fn no_fetcher_displays() {
        let err = WebError::NoFetcher;
        assert_eq!(err.to_string(), "no fetcher configured");
    }

    #[test]
    fn no_search_provider_displays() {
        let err = WebError::NoSearchProvider;
        assert_eq!(err.to_string(), "no search provider configured");
    }

    #[test]
    fn unsupported_scheme_displays() {
        let err = WebError::UnsupportedScheme("ftp".into());
        assert_eq!(err.to_string(), "unsupported scheme: ftp");
    }

    #[test]
    fn empty_query_displays() {
        let err = WebError::EmptyQuery;
        assert_eq!(err.to_string(), "empty query");
    }
}
