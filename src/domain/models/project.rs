use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Suspended,
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectStatus::Active => write!(f, "active"),
            ProjectStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::str::FromStr for ProjectStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "active" => Ok(ProjectStatus::Active),
            "suspended" => Ok(ProjectStatus::Suspended),
            other => Err(format!("unknown project status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: ProjectStatus::Active,
            created_at: Utc::now(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == ProjectStatus::Active
    }

    pub fn is_suspended(&self) -> bool {
        self.status == ProjectStatus::Suspended
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_active_status() {
        let p = Project::new("test");
        assert_eq!(p.status, ProjectStatus::Active);
    }

    #[test]
    fn project_status_serializes_lowercase() {
        let json = serde_json::to_string(&ProjectStatus::Suspended).unwrap();
        assert_eq!(json, "\"suspended\"");
    }

    #[test]
    fn project_status_roundtrips() {
        let active: ProjectStatus = serde_json::from_str("\"active\"").unwrap();
        assert_eq!(active, ProjectStatus::Active);
        let suspended: ProjectStatus = serde_json::from_str("\"suspended\"").unwrap();
        assert_eq!(suspended, ProjectStatus::Suspended);
    }
}
