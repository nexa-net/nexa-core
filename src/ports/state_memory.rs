use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::models::*;
use crate::error::{NexaError, Result};
use super::state::StateStore;

pub struct InMemoryStore {
    projects: Mutex<HashMap<String, Project>>,
    deployments: Mutex<HashMap<Uuid, Deployment>>,
    pods: Mutex<HashMap<Uuid, Pod>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            projects: Mutex::new(HashMap::new()),
            deployments: Mutex::new(HashMap::new()),
            pods: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl StateStore for InMemoryStore {
    async fn insert_project(&self, project: &Project) -> Result<()> {
        let mut map = self.projects.lock().unwrap();
        if map.contains_key(&project.name) {
            return Err(NexaError::InvalidSpec(format!(
                "project '{}' already exists",
                project.name
            )));
        }
        map.insert(project.name.clone(), project.clone());
        Ok(())
    }

    async fn get_project(&self, name: &str) -> Result<Option<Project>> {
        let map = self.projects.lock().unwrap();
        Ok(map.get(name).cloned())
    }

    async fn list_projects(&self) -> Result<Vec<Project>> {
        let map = self.projects.lock().unwrap();
        Ok(map.values().cloned().collect())
    }

    async fn update_project_status(&self, name: &str, status: ProjectStatus) -> Result<()> {
        let mut map = self.projects.lock().unwrap();
        match map.get_mut(name) {
            Some(p) => {
                p.status = status;
                Ok(())
            }
            None => Err(NexaError::ProjectNotFound(name.to_string())),
        }
    }

    async fn delete_project(&self, name: &str) -> Result<()> {
        let mut map = self.projects.lock().unwrap();
        map.remove(name);
        Ok(())
    }

    async fn insert_deployment(&self, deployment: &Deployment) -> Result<()> {
        let mut map = self.deployments.lock().unwrap();
        map.insert(deployment.id, deployment.clone());
        Ok(())
    }

    async fn get_deployment(&self, project: &str, name: &str) -> Result<Option<Deployment>> {
        let map = self.deployments.lock().unwrap();
        let found = map
            .values()
            .find(|d| d.project() == project && d.name() == name)
            .cloned();
        Ok(found)
    }

    async fn list_deployments(&self, project: Option<&str>) -> Result<Vec<Deployment>> {
        let map = self.deployments.lock().unwrap();
        let result = map
            .values()
            .filter(|d| match project {
                Some(p) => d.project() == p,
                None => true,
            })
            .cloned()
            .collect();
        Ok(result)
    }

    async fn update_deployment(&self, deployment: &Deployment) -> Result<()> {
        let mut map = self.deployments.lock().unwrap();
        map.insert(deployment.id, deployment.clone());
        Ok(())
    }

    async fn delete_deployment(&self, id: &Uuid) -> Result<()> {
        let mut map = self.deployments.lock().unwrap();
        map.remove(id);
        Ok(())
    }

    async fn insert_pod(&self, pod: &Pod) -> Result<()> {
        let mut map = self.pods.lock().unwrap();
        map.insert(pod.id, pod.clone());
        Ok(())
    }

    async fn list_pods(&self, project: Option<&str>) -> Result<Vec<Pod>> {
        let map = self.pods.lock().unwrap();
        let result = map
            .values()
            .filter(|p| match project {
                Some(proj) => p.project == proj,
                None => true,
            })
            .cloned()
            .collect();
        Ok(result)
    }

    async fn update_pod(&self, pod: &Pod) -> Result<()> {
        let mut map = self.pods.lock().unwrap();
        map.insert(pod.id, pod.clone());
        Ok(())
    }

    async fn delete_pod(&self, id: &Uuid) -> Result<()> {
        let mut map = self.pods.lock().unwrap();
        map.remove(id);
        Ok(())
    }

    async fn pods_by_deployment(&self, deployment_id: &Uuid) -> Result<Vec<Pod>> {
        let map = self.pods.lock().unwrap();
        let result = map
            .values()
            .filter(|p| p.deployment_id == *deployment_id)
            .cloned()
            .collect();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_get_project() {
        let store = InMemoryStore::new();
        let project = Project::new("myapp");

        store.insert_project(&project).await.unwrap();
        let fetched = store.get_project("myapp").await.unwrap();

        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "myapp");
    }

    #[tokio::test]
    async fn duplicate_project_errors() {
        let store = InMemoryStore::new();
        let project = Project::new("myapp");

        store.insert_project(&project).await.unwrap();
        let result = store.insert_project(&project).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_project_status_test() {
        let store = InMemoryStore::new();
        let project = Project::new("myapp");
        store.insert_project(&project).await.unwrap();

        store
            .update_project_status("myapp", ProjectStatus::Suspended)
            .await
            .unwrap();

        let fetched = store.get_project("myapp").await.unwrap().unwrap();
        assert_eq!(fetched.status, ProjectStatus::Suspended);
    }

    #[tokio::test]
    async fn insert_and_list_deployments() {
        let store = InMemoryStore::new();
        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 2,
            image: "nginx:latest".into(),
            ports: vec![],
            env: std::collections::HashMap::new(),
            secrets: vec![],
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };
        let deployment = Deployment::from_spec(spec);

        store.insert_deployment(&deployment).await.unwrap();

        let all = store.list_deployments(None).await.unwrap();
        assert_eq!(all.len(), 1);

        let filtered = store.list_deployments(Some("myapp")).await.unwrap();
        assert_eq!(filtered.len(), 1);

        let empty = store.list_deployments(Some("other")).await.unwrap();
        assert_eq!(empty.len(), 0);
    }

    #[tokio::test]
    async fn insert_and_query_pods() {
        let store = InMemoryStore::new();
        let deployment_id = Uuid::new_v4();

        let pod = Pod::new(deployment_id, "myapp", "api", 0, "nginx:latest");
        store.insert_pod(&pod).await.unwrap();

        let by_deployment = store.pods_by_deployment(&deployment_id).await.unwrap();
        assert_eq!(by_deployment.len(), 1);
        assert_eq!(by_deployment[0].restart_count, 0);

        let by_project = store.list_pods(Some("myapp")).await.unwrap();
        assert_eq!(by_project.len(), 1);
    }

    #[tokio::test]
    async fn delete_pod_test() {
        let store = InMemoryStore::new();
        let pod = Pod::new(Uuid::new_v4(), "myapp", "api", 0, "nginx:latest");
        let pod_id = pod.id;

        store.insert_pod(&pod).await.unwrap();
        store.delete_pod(&pod_id).await.unwrap();

        let all = store.list_pods(None).await.unwrap();
        assert_eq!(all.len(), 0);
    }
}
