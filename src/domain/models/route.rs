use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TlsMode {
    None,
    Auto,
    Manual,
}

impl std::fmt::Display for TlsMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsMode::None => write!(f, "none"),
            TlsMode::Auto => write!(f, "auto"),
            TlsMode::Manual => write!(f, "manual"),
        }
    }
}

impl std::str::FromStr for TlsMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(TlsMode::None),
            "auto" => Ok(TlsMode::Auto),
            "manual" => Ok(TlsMode::Manual),
            other => Err(format!("unknown TLS mode: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub domain: String,
    pub project: String,
    pub deployment: String,
    pub tls_mode: TlsMode,
    pub created_at: DateTime<Utc>,
}

impl Route {
    pub fn new(domain: &str, project: &str, deployment: &str, tls_mode: TlsMode) -> Self {
        Self {
            domain: domain.to_string(),
            project: project.to_string(),
            deployment: deployment.to_string(),
            tls_mode,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub domain: String,
    pub cert_pem: Vec<u8>,
    pub key_pem_enc: Vec<u8>,
    pub key_nonce: Vec<u8>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub acme_account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubnetAllocation {
    pub node_id: String,
    pub project: String,
    pub subnet: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_new_sets_fields() {
        let route = Route::new("api.example.com", "ecommerce", "api", TlsMode::Auto);
        assert_eq!(route.domain, "api.example.com");
        assert_eq!(route.project, "ecommerce");
        assert_eq!(route.deployment, "api");
        assert_eq!(route.tls_mode, TlsMode::Auto);
    }

    #[test]
    fn tls_mode_display() {
        assert_eq!(TlsMode::None.to_string(), "none");
        assert_eq!(TlsMode::Auto.to_string(), "auto");
        assert_eq!(TlsMode::Manual.to_string(), "manual");
    }

    #[test]
    fn tls_mode_from_str() {
        assert_eq!("none".parse::<TlsMode>().unwrap(), TlsMode::None);
        assert_eq!("auto".parse::<TlsMode>().unwrap(), TlsMode::Auto);
        assert_eq!("manual".parse::<TlsMode>().unwrap(), TlsMode::Manual);
        assert_eq!("AUTO".parse::<TlsMode>().unwrap(), TlsMode::Auto);
        assert!("invalid".parse::<TlsMode>().is_err());
    }

    #[test]
    fn tls_mode_serializes_json() {
        let mode = TlsMode::Auto;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, r#""auto""#);
    }

    #[test]
    fn tls_mode_deserializes_json() {
        let mode: TlsMode = serde_json::from_str(r#""manual""#).unwrap();
        assert_eq!(mode, TlsMode::Manual);
    }

    #[test]
    fn route_serializes_json_roundtrip() {
        let route = Route::new("web.example.com", "webapp", "frontend", TlsMode::None);
        let json = serde_json::to_string(&route).unwrap();
        let deserialized: Route = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.domain, "web.example.com");
        assert_eq!(deserialized.tls_mode, TlsMode::None);
    }

    #[test]
    fn subnet_allocation_fields() {
        let alloc = SubnetAllocation {
            node_id: "node-1".into(),
            project: "ecommerce".into(),
            subnet: "172.20.1.0/24".into(),
        };
        assert_eq!(alloc.node_id, "node-1");
        assert_eq!(alloc.subnet, "172.20.1.0/24");
    }

    #[test]
    fn certificate_fields() {
        let cert = Certificate {
            domain: "api.example.com".into(),
            cert_pem: b"CERT".to_vec(),
            key_pem_enc: b"KEY".to_vec(),
            key_nonce: b"NONCE".to_vec(),
            issued_at: Utc::now(),
            expires_at: Utc::now(),
            acme_account: Some("acct-123".into()),
        };
        assert_eq!(cert.domain, "api.example.com");
        assert!(cert.acme_account.is_some());
    }
}
