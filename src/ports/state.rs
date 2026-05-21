use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::models::{
    Deployment, Pod, Project, ProjectStatus,
};
use crate::error::Result;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn insert_project(&self, project: &Project) -> Result<()>;
    async fn get_project(&self, name: &str) -> Result<Option<Project>>;
    async fn list_projects(&self) -> Result<Vec<Project>>;
    async fn update_project_status(&self, name: &str, status: ProjectStatus) -> Result<()>;
    async fn delete_project(&self, name: &str) -> Result<()>;

    async fn insert_deployment(&self, deployment: &Deployment) -> Result<()>;
    async fn get_deployment(&self, project: &str, name: &str) -> Result<Option<Deployment>>;
    async fn list_deployments(&self, project: Option<&str>) -> Result<Vec<Deployment>>;
    async fn update_deployment(&self, deployment: &Deployment) -> Result<()>;
    async fn delete_deployment(&self, id: &Uuid) -> Result<()>;

    async fn insert_pod(&self, pod: &Pod) -> Result<()>;
    async fn list_pods(&self, project: Option<&str>) -> Result<Vec<Pod>>;
    async fn update_pod(&self, pod: &Pod) -> Result<()>;
    async fn delete_pod(&self, id: &Uuid) -> Result<()>;
    async fn pods_by_deployment(&self, deployment_id: &Uuid) -> Result<Vec<Pod>>;
}
