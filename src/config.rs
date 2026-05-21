use std::path::Path;

use regex::Regex;

use crate::domain::models::DeploymentSpec;
use crate::error::{NexaError, Result};

pub fn parse_deployment_file(path: &Path) -> Result<DeploymentSpec> {
    let content = std::fs::read_to_string(path)?;
    parse_deployment(&content)
}

pub fn parse_deployment(yaml: &str) -> Result<DeploymentSpec> {
    let spec: DeploymentSpec =
        serde_yaml::from_str(yaml).map_err(|e| NexaError::InvalidSpec(e.to_string()))?;
    validate_spec(&spec)?;
    Ok(spec)
}

fn validate_dns_name(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Err(NexaError::InvalidSpec(format!("{field} is required")));
    }
    if value.len() > 63 {
        return Err(NexaError::InvalidSpec(format!(
            "{field} must be at most 63 characters, got {}",
            value.len()
        )));
    }
    let dns_re = Regex::new(r"^[a-z0-9][a-z0-9-]*$").unwrap();
    if !dns_re.is_match(value) {
        return Err(NexaError::InvalidSpec(format!(
            "{field} must be DNS-safe: start with [a-z0-9], then [a-z0-9-] only (got '{value}')"
        )));
    }
    Ok(())
}

fn validate_spec(spec: &DeploymentSpec) -> Result<()> {
    validate_dns_name(&spec.project, "project")?;
    validate_dns_name(&spec.deployment.name, "deployment name")?;

    if spec.image.is_empty() {
        return Err(NexaError::InvalidSpec("image is required".into()));
    }
    if spec.replicas == 0 {
        return Err(NexaError::InvalidSpec(
            "replicas must be at least 1".into(),
        ));
    }

    for &port in &spec.ports {
        if port == 0 {
            return Err(NexaError::InvalidSpec(
                "port must be between 1 and 65535, got 0".into(),
            ));
        }
    }

    if let Some(ref res) = spec.resources {
        validate_resource_memory(&res.memory)?;
        if res.cpu <= 0.0 {
            return Err(NexaError::InvalidSpec(
                "resources.cpu must be greater than 0".into(),
            ));
        }
    }

    Ok(())
}

fn validate_resource_memory(memory: &str) -> Result<()> {
    if memory.is_empty() {
        return Err(NexaError::InvalidSpec(
            "resources.memory is required when resources is specified".into(),
        ));
    }
    let mem_re = Regex::new(r"^[0-9]+[kmgKMG]$").unwrap();
    if !mem_re.is_match(memory) {
        return Err(NexaError::InvalidSpec(format!(
            "resources.memory must match format like '512m', '1g', '256k' (got '{memory}')"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_spec() {
        let yaml = r#"
project: myapp

deployment:
  name: api

image: nginx:latest
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert_eq!(spec.project, "myapp");
        assert_eq!(spec.deployment.name, "api");
        assert_eq!(spec.image, "nginx:latest");
        assert_eq!(spec.replicas, 1);
    }

    #[test]
    fn parse_full_spec() {
        let yaml = r#"
project: ecommerce

deployment:
  name: api

replicas: 3
image: ghcr.io/company/api:latest

ports:
  - 3000

network:
  public: true
  domain: api.example.com
  https: true

env:
  DATABASE_URL: "postgres://localhost/db"
  REDIS_URL: "redis://localhost"

healthcheck:
  path: /health
  interval: 10s
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert_eq!(spec.replicas, 3);
        assert_eq!(spec.ports, vec![3000]);
        assert!(spec.network.as_ref().unwrap().public);
        assert_eq!(spec.env.len(), 2);
        assert_eq!(spec.healthcheck.as_ref().unwrap().path, "/health");
    }

    #[test]
    fn reject_empty_project() {
        let yaml = r#"
project: ""
deployment:
  name: api
image: nginx
"#;
        assert!(parse_deployment(yaml).is_err());
    }

    #[test]
    fn reject_zero_replicas() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx
replicas: 0
"#;
        assert!(parse_deployment(yaml).is_err());
    }

    #[test]
    fn parse_secrets_field() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
secrets:
  - DATABASE_URL
  - STRIPE_KEY
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert_eq!(spec.secrets, vec!["DATABASE_URL", "STRIPE_KEY"]);
    }

    #[test]
    fn parse_empty_secrets_defaults_to_empty() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert!(spec.secrets.is_empty());
    }

    #[test]
    fn parse_resources_field() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
resources:
  memory: 512m
  cpu: 0.5
"#;
        let spec = parse_deployment(yaml).unwrap();
        let res = spec.resources.unwrap();
        assert_eq!(res.memory, "512m");
        assert!((res.cpu - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_no_resources_defaults_to_none() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert!(spec.resources.is_none());
    }

    #[test]
    fn parse_named_volume() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
volumes:
  - name: data
    mount: /app/data
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert_eq!(spec.volumes.len(), 1);
        assert_eq!(spec.volumes[0].mount_point(), "/app/data");
        assert_eq!(spec.volumes[0].source_name(), "data");
        assert!(!spec.volumes[0].is_read_only());
    }

    #[test]
    fn parse_bind_mount_volume() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
volumes:
  - path: /host/uploads
    mount: /app/uploads
    readonly: true
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert_eq!(spec.volumes.len(), 1);
        assert_eq!(spec.volumes[0].mount_point(), "/app/uploads");
        assert_eq!(spec.volumes[0].source_name(), "/host/uploads");
        assert!(spec.volumes[0].is_read_only());
    }

    #[test]
    fn parse_mixed_volumes() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
volumes:
  - name: data
    mount: /app/data
  - path: /host/uploads
    mount: /app/uploads
    readonly: true
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert_eq!(spec.volumes.len(), 2);
        assert_eq!(spec.volumes[0].source_name(), "data");
        assert_eq!(spec.volumes[1].source_name(), "/host/uploads");
    }

    #[test]
    fn parse_bind_mount_readonly_defaults_false() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
volumes:
  - path: /host/data
    mount: /app/data
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert!(!spec.volumes[0].is_read_only());
    }

    #[test]
    fn reject_uppercase_project_name() {
        let yaml = r#"
project: MyApp
deployment:
  name: api
image: nginx
"#;
        let err = parse_deployment(yaml).unwrap_err();
        assert!(err.to_string().contains("DNS-safe"));
    }

    #[test]
    fn reject_project_starting_with_hyphen() {
        let yaml = r#"
project: -myapp
deployment:
  name: api
image: nginx
"#;
        let err = parse_deployment(yaml).unwrap_err();
        assert!(err.to_string().contains("DNS-safe"));
    }

    #[test]
    fn reject_project_name_too_long() {
        let long_name = "a".repeat(64);
        let yaml = format!(
            r#"
project: {long_name}
deployment:
  name: api
image: nginx
"#
        );
        let err = parse_deployment(&yaml).unwrap_err();
        assert!(err.to_string().contains("63 characters"));
    }

    #[test]
    fn reject_deployment_name_with_underscore() {
        let yaml = r#"
project: myapp
deployment:
  name: my_api
image: nginx
"#;
        let err = parse_deployment(yaml).unwrap_err();
        assert!(err.to_string().contains("DNS-safe"));
    }

    #[test]
    fn accept_valid_dns_names() {
        let yaml = r#"
project: my-app-123
deployment:
  name: api-v2
image: nginx:latest
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert_eq!(spec.project, "my-app-123");
        assert_eq!(spec.deployment.name, "api-v2");
    }

    #[test]
    fn reject_port_zero() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx
ports:
  - 0
"#;
        let err = parse_deployment(yaml).unwrap_err();
        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn accept_valid_port_range() {
        let yaml = r#"
project: myapp
deployment:
  name: api
image: nginx
ports:
  - 1
  - 8080
  - 65535
"#;
        let spec = parse_deployment(yaml).unwrap();
        assert_eq!(spec.ports, vec![1, 8080, 65535]);
    }
}
