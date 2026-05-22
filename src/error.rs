use thiserror::Error;

#[derive(Debug, Error)]
pub enum NexaError {
    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("deployment not found: {0}")]
    DeploymentNotFound(String),

    #[error("pod not found: {0}")]
    PodNotFound(String),

    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("container runtime error: {0}")]
    Runtime(String),

    #[error("invalid deployment spec: {0}")]
    InvalidSpec(String),

    #[error("port conflict: port {0} is already in use")]
    PortConflict(u16),

    #[error("image pull failed: {0}")]
    ImagePull(String),

    #[error("health check failed for {0}")]
    HealthCheckFailed(String),

    #[error("state store error: {0}")]
    StateStore(String),

    #[error("secret error: {0}")]
    Secret(String),

    #[error("project is suspended: {0}")]
    ProjectSuspended(String),

    #[error("project not empty: {0}")]
    ProjectNotEmpty(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("scheduling failed: {0}")]
    SchedulingFailed(String),

    #[error("route not found: {0}")]
    RouteNotFound(String),

    #[error("route already exists: {0}")]
    RouteAlreadyExists(String),

    #[error("proxy error: {0}")]
    Proxy(String),

    #[error("certificate error: {0}")]
    Certificate(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("dns error: {0}")]
    Dns(String),
}

pub type Result<T> = std::result::Result<T, NexaError>;
