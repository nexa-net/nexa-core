use std::path::Path;

use crate::error::{NexaError, Result};
use crate::domain::models::DeploymentSpec;

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

fn validate_spec(spec: &DeploymentSpec) -> Result<()> {
    if spec.project.is_empty() {
        return Err(NexaError::InvalidSpec("project name is required".into()));
    }
    if spec.deployment.name.is_empty() {
        return Err(NexaError::InvalidSpec("deployment name is required".into()));
    }
    if spec.image.is_empty() {
        return Err(NexaError::InvalidSpec("image is required".into()));
    }
    if spec.replicas == 0 {
        return Err(NexaError::InvalidSpec(
            "replicas must be at least 1".into(),
        ));
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
}
