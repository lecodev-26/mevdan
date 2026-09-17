//! `WebUrl` — URL tipada con parsing básico.

use crate::error::{WebError, WebResult};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    pub fn display_name(&self) -> &'static str {
        match self {
            Scheme::Http => "http",
            Scheme::Https => "https",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }

    pub fn parse(s: &str) -> WebResult<Self> {
        match s.to_lowercase().as_str() {
            "http" => Ok(Scheme::Http),
            "https" => Ok(Scheme::Https),
            other => Err(WebError::UnsupportedScheme(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WebUrl {
    pub scheme: Scheme,
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
}

impl WebUrl {
    pub fn new(scheme: Scheme, host: impl Into<String>) -> Self {
        Self {
            scheme,
            host: host.into(),
            port: None,
            path: String::new(),
            query: None,
            fragment: None,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn with_fragment(mut self, fragment: impl Into<String>) -> Self {
        self.fragment = Some(fragment.into());
        self
    }

    pub fn effective_port(&self) -> u16 {
        self.port.unwrap_or_else(|| self.scheme.default_port())
    }

    pub fn authority(&self) -> String {
        match self.port {
            Some(p) => format!("{}:{}", self.host, p),
            None => self.host.clone(),
        }
    }

    pub fn normalized_path(&self) -> String {
        if self.path.is_empty() {
            "/".to_string()
        } else if self.path.starts_with('/') {
            self.path.clone()
        } else {
            format!("/{}", self.path)
        }
    }

    pub fn request_target(&self) -> String {
        let mut s = self.normalized_path();
        if let Some(q) = &self.query {
            s.push('?');
            s.push_str(q);
        }
        s
    }
}

impl FromStr for WebUrl {
    type Err = WebError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(WebError::InvalidUrl("empty".into()));
        }

        let (scheme_str, rest) = s
            .split_once("://")
            .ok_or_else(|| WebError::InvalidUrl(format!("missing scheme: {}", s)))?;

        let scheme = Scheme::parse(scheme_str)?;

        let (rest, fragment) = match rest.split_once('#') {
            Some((r, f)) => (r, Some(f.to_string())),
            None => (rest, None),
        };

        let (rest, query) = match rest.split_once('?') {
            Some((r, q)) => (r, Some(q.to_string())),
            None => (rest, None),
        };

        let (authority, path) = match rest.find('/') {
            Some(idx) => (&rest[..idx], rest[idx..].to_string()),
            None => (rest, String::new()),
        };

        if authority.is_empty() {
            return Err(WebError::InvalidUrl(format!("empty host: {}", s)));
        }

        let (host, port) = match authority.rsplit_once(':') {
            Some((h, p)) => {
                let port: u16 = p
                    .parse()
                    .map_err(|_| WebError::InvalidUrl(format!("invalid port: {}", p)))?;
                (h.to_string(), Some(port))
            }
            None => (authority.to_string(), None),
        };

        if host.is_empty() {
            return Err(WebError::InvalidUrl(format!("empty host: {}", s)));
        }

        Ok(Self {
            scheme,
            host,
            port,
            path,
            query,
            fragment,
        })
    }
}

impl fmt::Display for WebUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme.display_name(), self.authority())?;
        if !self.path.is_empty() {
            write!(f, "{}", self.path)?;
        }
        if let Some(q) = &self.query {
            write!(f, "?{}", q)?;
        }
        if let Some(fr) = &self.fragment {
            write!(f, "#{}", fr)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_display_names() {
        assert_eq!(Scheme::Http.display_name(), "http");
        assert_eq!(Scheme::Https.display_name(), "https");
    }

    #[test]
    fn scheme_default_ports() {
        assert_eq!(Scheme::Http.default_port(), 80);
        assert_eq!(Scheme::Https.default_port(), 443);
    }

    #[test]
    fn scheme_parse() {
        assert_eq!(Scheme::parse("http").unwrap(), Scheme::Http);
        assert_eq!(Scheme::parse("HTTPS").unwrap(), Scheme::Https);
        assert!(Scheme::parse("ftp").is_err());
    }

    #[test]
    fn url_parse_simple() {
        let url: WebUrl = "https://example.com".parse().unwrap();
        assert_eq!(url.scheme, Scheme::Https);
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, None);
        assert_eq!(url.path, "");
    }

    #[test]
    fn url_parse_with_path() {
        let url: WebUrl = "https://example.com/api/v1/users".parse().unwrap();
        assert_eq!(url.host, "example.com");
        assert_eq!(url.path, "/api/v1/users");
    }

    #[test]
    fn url_parse_with_port() {
        let url: WebUrl = "http://localhost:8080/api".parse().unwrap();
        assert_eq!(url.scheme, Scheme::Http);
        assert_eq!(url.host, "localhost");
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/api");
    }

    #[test]
    fn url_parse_with_query() {
        let url: WebUrl = "https://example.com/search?q=rust&lang=en".parse().unwrap();
        assert_eq!(url.query.as_deref(), Some("q=rust&lang=en"));
    }

    #[test]
    fn url_parse_with_fragment() {
        let url: WebUrl = "https://docs.rs/serde#examples".parse().unwrap();
        assert_eq!(url.host, "docs.rs");
        assert_eq!(url.path, "/serde");
        assert_eq!(url.fragment.as_deref(), Some("examples"));
    }

    #[test]
    fn url_parse_all_parts() {
        let url: WebUrl = "https://api.example.com:8443/v1/search?q=test#top"
            .parse()
            .unwrap();
        assert_eq!(url.scheme, Scheme::Https);
        assert_eq!(url.host, "api.example.com");
        assert_eq!(url.port, Some(8443));
        assert_eq!(url.path, "/v1/search");
        assert_eq!(url.query.as_deref(), Some("q=test"));
        assert_eq!(url.fragment.as_deref(), Some("top"));
    }

    #[test]
    fn url_parse_fails_without_scheme() {
        let err = "example.com/path".parse::<WebUrl>().unwrap_err();
        assert!(matches!(err, WebError::InvalidUrl(_)));
    }

    #[test]
    fn url_parse_fails_with_unsupported_scheme() {
        let err = "ftp://example.com".parse::<WebUrl>().unwrap_err();
        assert!(matches!(err, WebError::UnsupportedScheme(_)));
    }

    #[test]
    fn url_parse_fails_with_empty() {
        let err = "".parse::<WebUrl>().unwrap_err();
        assert!(matches!(err, WebError::InvalidUrl(_)));
    }

    #[test]
    fn url_parse_fails_with_bad_port() {
        let err = "http://example.com:abc/".parse::<WebUrl>().unwrap_err();
        assert!(matches!(err, WebError::InvalidUrl(_)));
    }

    #[test]
    fn effective_port_uses_default() {
        let url: WebUrl = "https://example.com".parse().unwrap();
        assert_eq!(url.effective_port(), 443);

        let url: WebUrl = "http://example.com".parse().unwrap();
        assert_eq!(url.effective_port(), 80);

        let url: WebUrl = "https://example.com:8443".parse().unwrap();
        assert_eq!(url.effective_port(), 8443);
    }

    #[test]
    fn authority_format() {
        let url: WebUrl = "https://example.com".parse().unwrap();
        assert_eq!(url.authority(), "example.com");

        let url: WebUrl = "https://example.com:8443".parse().unwrap();
        assert_eq!(url.authority(), "example.com:8443");
    }

    #[test]
    fn normalized_path() {
        let url: WebUrl = "https://example.com".parse().unwrap();
        assert_eq!(url.normalized_path(), "/");

        let url: WebUrl = "https://example.com/api".parse().unwrap();
        assert_eq!(url.normalized_path(), "/api");
    }

    #[test]
    fn request_target() {
        let url: WebUrl = "https://example.com/api?q=test".parse().unwrap();
        assert_eq!(url.request_target(), "/api?q=test");

        let url: WebUrl = "https://example.com".parse().unwrap();
        assert_eq!(url.request_target(), "/");
    }

    #[test]
    fn display_roundtrips() {
        let original = "https://api.example.com:8443/v1/search?q=test#top";
        let url: WebUrl = original.parse().unwrap();
        assert_eq!(url.to_string(), original);
    }

    #[test]
    fn display_minimal() {
        let url: WebUrl = "https://example.com".parse().unwrap();
        assert_eq!(url.to_string(), "https://example.com");
    }

    #[test]
    fn url_serializes() {
        let url: WebUrl = "https://example.com/api?q=x".parse().unwrap();
        let json = serde_json::to_string(&url).unwrap();
        let back: WebUrl = serde_json::from_str(&json).unwrap();
        assert_eq!(back, url);
    }

    #[test]
    fn build_url_manually() {
        let url = WebUrl::new(Scheme::Https, "api.example.com")
            .with_port(8443)
            .with_path("/v1/search")
            .with_query("q=test");
        assert_eq!(url.effective_port(), 8443);
        assert_eq!(
            url.to_string(),
            "https://api.example.com:8443/v1/search?q=test"
        );
    }
}
