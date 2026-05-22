use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub domain: String,
    pub upstream: Vec<Upstream>,
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upstream {
    pub address: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum TlsConfig {
    None,
    Auto { email: String },
    Manual { cert: PathBuf, key: PathBuf },
}

#[async_trait]
pub trait ProxyBackend: Send + Sync {
    async fn apply_routes(&self, routes: &[RouteConfig]) -> Result<()>;
    async fn remove_route(&self, domain: &str) -> Result<()>;
    async fn reload(&self) -> Result<()>;
    async fn health(&self) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn ProxyBackend) {}
    }

    #[test]
    fn route_config_serializes_to_json() {
        let config = RouteConfig {
            domain: "api.example.com".into(),
            upstream: vec![
                Upstream {
                    address: "10.0.0.1:3000".into(),
                    weight: 1,
                },
                Upstream {
                    address: "10.0.0.2:3000".into(),
                    weight: 1,
                },
            ],
            tls: TlsConfig::Auto {
                email: "admin@example.com".into(),
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("api.example.com"));
        assert!(json.contains("10.0.0.1:3000"));
        assert!(json.contains("admin@example.com"));
    }

    #[test]
    fn tls_config_none_serializes() {
        let tls = TlsConfig::None;
        let json = serde_json::to_string(&tls).unwrap();
        assert!(json.contains("none"));
    }

    #[test]
    fn tls_config_manual_serializes() {
        let tls = TlsConfig::Manual {
            cert: PathBuf::from("/etc/certs/cert.pem"),
            key: PathBuf::from("/etc/certs/key.pem"),
        };
        let json = serde_json::to_string(&tls).unwrap();
        assert!(json.contains("manual"));
        assert!(json.contains("cert.pem"));
    }

    #[test]
    fn upstream_default_weight() {
        let up = Upstream {
            address: "10.0.0.1:8080".into(),
            weight: 1,
        };
        assert_eq!(up.weight, 1);
    }

    #[test]
    fn route_config_deserializes_from_json() {
        let json = r#"{
            "domain": "web.example.com",
            "upstream": [{"address": "10.0.0.5:80", "weight": 3}],
            "tls": {"mode": "none"}
        }"#;
        let config: RouteConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.domain, "web.example.com");
        assert_eq!(config.upstream.len(), 1);
        assert_eq!(config.upstream[0].weight, 3);
        assert!(matches!(config.tls, TlsConfig::None));
    }
}
