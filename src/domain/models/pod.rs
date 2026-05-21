use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pod {
    pub id: Uuid,
    pub deployment_id: Uuid,
    pub project: String,
    pub deployment_name: String,
    pub replica_index: u32,
    pub container_id: Option<String>,
    pub status: PodStatus,
    pub image: String,
    pub restart_count: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PodStatus {
    Pending,
    Creating,
    Running,
    Stopping,
    Stopped,
    Failed,
    Restarting,
}

impl Pod {
    pub fn new(
        deployment_id: Uuid,
        project: &str,
        deployment_name: &str,
        replica_index: u32,
        image: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            deployment_id,
            project: project.to_string(),
            deployment_name: deployment_name.to_string(),
            replica_index,
            container_id: None,
            status: PodStatus::Pending,
            image: image.to_string(),
            restart_count: 0,
            created_at: Utc::now(),
        }
    }

    pub fn container_name(&self) -> String {
        format!(
            "nexa-{}-{}-{}",
            self.project, self.deployment_name, self.replica_index
        )
    }
}

impl std::fmt::Display for PodStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PodStatus::Pending => write!(f, "Pending"),
            PodStatus::Creating => write!(f, "Creating"),
            PodStatus::Running => write!(f, "Running"),
            PodStatus::Stopping => write!(f, "Stopping"),
            PodStatus::Stopped => write!(f, "Stopped"),
            PodStatus::Failed => write!(f, "Failed"),
            PodStatus::Restarting => write!(f, "Restarting"),
        }
    }
}

impl std::str::FromStr for PodStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(PodStatus::Pending),
            "creating" => Ok(PodStatus::Creating),
            "running" => Ok(PodStatus::Running),
            "stopping" => Ok(PodStatus::Stopping),
            "stopped" => Ok(PodStatus::Stopped),
            "failed" => Ok(PodStatus::Failed),
            "restarting" => Ok(PodStatus::Restarting),
            other => Err(format!("unknown pod status: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pod_has_zero_restart_count() {
        let pod = Pod::new(
            Uuid::new_v4(),
            "proj",
            "deploy",
            0,
            "nginx:latest",
        );
        assert_eq!(pod.restart_count, 0);
    }
}
