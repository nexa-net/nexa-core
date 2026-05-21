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
    nodes: Mutex<HashMap<Uuid, Node>>,
    cluster_config: Mutex<HashMap<String, String>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            projects: Mutex::new(HashMap::new()),
            deployments: Mutex::new(HashMap::new()),
            pods: Mutex::new(HashMap::new()),
            nodes: Mutex::new(HashMap::new()),
            cluster_config: Mutex::new(HashMap::new()),
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

    async fn insert_node(&self, node: &Node) -> Result<()> {
        let mut map = self.nodes.lock().unwrap();
        if map.values().any(|n| n.name == node.name) {
            return Err(NexaError::InvalidSpec(format!(
                "node '{}' already exists",
                node.name
            )));
        }
        map.insert(node.id, node.clone());
        Ok(())
    }

    async fn get_node(&self, id: &Uuid) -> Result<Option<Node>> {
        let map = self.nodes.lock().unwrap();
        Ok(map.get(id).cloned())
    }

    async fn get_node_by_name(&self, name: &str) -> Result<Option<Node>> {
        let map = self.nodes.lock().unwrap();
        Ok(map.values().find(|n| n.name == name).cloned())
    }

    async fn list_nodes(&self) -> Result<Vec<Node>> {
        let map = self.nodes.lock().unwrap();
        Ok(map.values().cloned().collect())
    }

    async fn update_node(&self, node: &Node) -> Result<()> {
        let mut map = self.nodes.lock().unwrap();
        if !map.contains_key(&node.id) {
            return Err(NexaError::NodeNotFound(node.id.to_string()));
        }
        map.insert(node.id, node.clone());
        Ok(())
    }

    async fn delete_node(&self, id: &Uuid) -> Result<()> {
        let mut map = self.nodes.lock().unwrap();
        map.remove(id);
        Ok(())
    }

    async fn get_cluster_config(&self, key: &str) -> Result<Option<String>> {
        let map = self.cluster_config.lock().unwrap();
        Ok(map.get(key).cloned())
    }

    async fn set_cluster_config(&self, key: &str, value: &str) -> Result<()> {
        let mut map = self.cluster_config.lock().unwrap();
        map.insert(key.to_string(), value.to_string());
        Ok(())
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

    fn sample_resources() -> NodeResources {
        NodeResources {
            cpu_cores: 4.0,
            memory_bytes: 8_589_934_592,
            cpu_available: 3.5,
            memory_available: 7_000_000_000,
            running_pods: 2,
        }
    }

    #[tokio::test]
    async fn insert_and_get_node() {
        let store = InMemoryStore::new();
        let node = Node::new(
            "worker-1".into(),
            "192.168.1.1:9000".into(),
            NodeRole::Worker,
            sample_resources(),
        );
        let node_id = node.id;

        store.insert_node(&node).await.unwrap();
        let fetched = store.get_node(&node_id).await.unwrap();

        assert!(fetched.is_some());
        let n = fetched.unwrap();
        assert_eq!(n.name, "worker-1");
        assert_eq!(n.address, "192.168.1.1:9000");
        assert_eq!(n.role, NodeRole::Worker);
    }

    #[tokio::test]
    async fn get_node_by_name() {
        let store = InMemoryStore::new();
        let node = Node::new(
            "master-1".into(),
            "10.0.0.1:9000".into(),
            NodeRole::Master,
            sample_resources(),
        );
        store.insert_node(&node).await.unwrap();

        let fetched = store.get_node_by_name("master-1").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, node.id);

        let missing = store.get_node_by_name("nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn list_nodes() {
        let store = InMemoryStore::new();
        let n1 = Node::new("w1".into(), "10.0.0.1:9000".into(), NodeRole::Worker, sample_resources());
        let n2 = Node::new("w2".into(), "10.0.0.2:9000".into(), NodeRole::Worker, sample_resources());

        store.insert_node(&n1).await.unwrap();
        store.insert_node(&n2).await.unwrap();

        let all = store.list_nodes().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn update_node() {
        let store = InMemoryStore::new();
        let mut node = Node::new(
            "worker-1".into(),
            "10.0.0.1:9000".into(),
            NodeRole::Worker,
            sample_resources(),
        );
        store.insert_node(&node).await.unwrap();

        node.status = NodeStatus::Draining;
        store.update_node(&node).await.unwrap();

        let fetched = store.get_node(&node.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, NodeStatus::Draining);

        // Updating a non-existent node should error
        let phantom = Node::new("ghost".into(), "0.0.0.0:0".into(), NodeRole::Worker, sample_resources());
        let result = store.update_node(&phantom).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_node() {
        let store = InMemoryStore::new();
        let node = Node::new("w1".into(), "10.0.0.1:9000".into(), NodeRole::Worker, sample_resources());
        let node_id = node.id;

        store.insert_node(&node).await.unwrap();
        store.delete_node(&node_id).await.unwrap();

        let fetched = store.get_node(&node_id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn cluster_config_roundtrip() {
        let store = InMemoryStore::new();

        // Initially empty
        let val = store.get_cluster_config("leader_id").await.unwrap();
        assert!(val.is_none());

        // Set and get
        store.set_cluster_config("leader_id", "node-abc").await.unwrap();
        let val = store.get_cluster_config("leader_id").await.unwrap();
        assert_eq!(val.as_deref(), Some("node-abc"));

        // Overwrite
        store.set_cluster_config("leader_id", "node-xyz").await.unwrap();
        let val = store.get_cluster_config("leader_id").await.unwrap();
        assert_eq!(val.as_deref(), Some("node-xyz"));
    }
}
