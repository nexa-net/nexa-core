use std::sync::Arc;
use std::collections::HashMap as StdHashMap;

use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::domain::models::*;
use crate::error::{NexaError, Result};
use crate::ports::runtime::{ContainerConfig, ContainerRuntime, LogStream};

pub enum Command {
    Deploy {
        spec: DeploymentSpec,
        reply: oneshot::Sender<Result<Deployment>>,
    },
    ListDeployments {
        project: Option<String>,
        reply: oneshot::Sender<Vec<Deployment>>,
    },
    ListPods {
        project: Option<String>,
        reply: oneshot::Sender<Vec<Pod>>,
    },
    CreateProject {
        name: String,
        reply: oneshot::Sender<Result<Project>>,
    },
    ListProjects {
        reply: oneshot::Sender<Vec<Project>>,
    },
    Stop {
        project: String,
        name: String,
        reply: oneshot::Sender<Result<()>>,
    },
    RemoveDeployment {
        project: String,
        name: String,
        reply: oneshot::Sender<Result<()>>,
    },
    Scale {
        project: String,
        name: String,
        replicas: u32,
        reply: oneshot::Sender<Result<Deployment>>,
    },
    PodLogs {
        project: String,
        name: String,
        tail: Option<u64>,
        reply: oneshot::Sender<Result<LogStream>>,
    },
}

#[derive(Clone)]
pub struct OrchestratorHandle {
    tx: mpsc::Sender<Command>,
}

impl OrchestratorHandle {
    pub async fn deploy(&self, spec: DeploymentSpec) -> Result<Deployment> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Deploy { spec, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn list_deployments(&self, project: Option<String>) -> Vec<Deployment> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Command::ListDeployments { project, reply }).await;
        rx.await.unwrap_or_default()
    }

    pub async fn list_pods(&self, project: Option<String>) -> Vec<Pod> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Command::ListPods { project, reply }).await;
        rx.await.unwrap_or_default()
    }

    pub async fn create_project(&self, name: String) -> Result<Project> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::CreateProject { name, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn list_projects(&self) -> Vec<Project> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Command::ListProjects { reply }).await;
        rx.await.unwrap_or_default()
    }

    pub async fn stop(&self, project: String, name: String) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Stop { project, name, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn remove_deployment(&self, project: String, name: String) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::RemoveDeployment { project, name, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn scale(&self, project: String, name: String, replicas: u32) -> Result<Deployment> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Scale { project, name, replicas, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn pod_logs(&self, project: String, name: String, tail: Option<u64>) -> Result<LogStream> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::PodLogs { project, name, tail, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }
}

pub struct Orchestrator {
    runtime: Arc<dyn ContainerRuntime>,
    projects: StdHashMap<String, Project>,
    deployments: StdHashMap<Uuid, Deployment>,
    pods: StdHashMap<Uuid, Pod>,
}

impl Orchestrator {
    pub fn spawn(runtime: Arc<dyn ContainerRuntime>) -> OrchestratorHandle {
        let (tx, rx) = mpsc::channel(256);
        tokio::spawn(async move {
            let mut orch = Self {
                runtime,
                projects: StdHashMap::new(),
                deployments: StdHashMap::new(),
                pods: StdHashMap::new(),
            };
            orch.run(rx).await;
        });
        OrchestratorHandle { tx }
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<Command>) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Deploy { spec, reply } => {
                    let result = self.handle_deploy(spec).await;
                    let _ = reply.send(result);
                }
                Command::ListDeployments { project, reply } => {
                    let _ = reply.send(self.handle_list_deployments(project.as_deref()));
                }
                Command::ListPods { project, reply } => {
                    let _ = reply.send(self.handle_list_pods(project.as_deref()));
                }
                Command::CreateProject { name, reply } => {
                    let _ = reply.send(self.handle_create_project(&name));
                }
                Command::ListProjects { reply } => {
                    let _ = reply.send(self.handle_list_projects());
                }
                Command::Stop { project, name, reply } => {
                    let result = self.handle_stop(&project, &name).await;
                    let _ = reply.send(result);
                }
                Command::RemoveDeployment { project, name, reply } => {
                    let result = self.handle_remove_deployment(&project, &name).await;
                    let _ = reply.send(result);
                }
                Command::Scale { project, name, replicas, reply } => {
                    let result = self.handle_scale(&project, &name, replicas).await;
                    let _ = reply.send(result);
                }
                Command::PodLogs { project, name, tail, reply } => {
                    let result = self.handle_pod_logs(&project, &name, tail).await;
                    let _ = reply.send(result);
                }
            }
        }
    }

    fn ensure_project(&mut self, name: &str) {
        if !self.projects.contains_key(name) {
            self.projects.insert(name.to_string(), Project::new(name));
        }
    }

    fn handle_create_project(&mut self, name: &str) -> Result<Project> {
        if self.projects.contains_key(name) {
            return Err(NexaError::InvalidSpec(format!("project '{name}' already exists")));
        }
        let project = Project::new(name);
        self.projects.insert(name.to_string(), project.clone());
        Ok(project)
    }

    fn handle_list_projects(&self) -> Vec<Project> {
        self.projects.values().cloned().collect()
    }

    async fn handle_deploy(&mut self, spec: DeploymentSpec) -> Result<Deployment> {
        self.ensure_project(&spec.project);

        let existing_id = self.find_deployment_id(&spec.project, &spec.deployment.name);

        if let Some(id) = existing_id {
            let deployment = self.deployments.get_mut(&id).unwrap();
            deployment.spec = spec.clone();
            deployment.updated_at = chrono::Utc::now();
            let id = deployment.id;
            self.reconcile_deployment(id).await?;
            return Ok(self.deployments[&id].clone());
        }

        let deployment = Deployment::from_spec(spec);
        let id = deployment.id;
        self.deployments.insert(id, deployment);
        self.reconcile_deployment(id).await?;
        Ok(self.deployments[&id].clone())
    }

    fn handle_list_deployments(&self, project: Option<&str>) -> Vec<Deployment> {
        self.deployments
            .values()
            .filter(|d| match project {
                Some(p) => d.project() == p,
                None => true,
            })
            .cloned()
            .collect()
    }

    fn handle_list_pods(&self, project: Option<&str>) -> Vec<Pod> {
        self.pods
            .values()
            .filter(|p| match project {
                Some(proj) => p.project == proj,
                None => true,
            })
            .cloned()
            .collect()
    }

    async fn handle_stop(&mut self, project: &str, name: &str) -> Result<()> {
        let deployment_id = self
            .find_deployment_id(project, name)
            .ok_or_else(|| NexaError::DeploymentNotFound(format!("{project}/{name}")))?;

        let pod_ids: Vec<Uuid> = self
            .pods
            .values()
            .filter(|p| p.deployment_id == deployment_id)
            .map(|p| p.id)
            .collect();

        for pod_id in &pod_ids {
            if let Some(pod) = self.pods.get(pod_id) {
                if let Some(cid) = &pod.container_id {
                    let _ = self.runtime.stop_container(cid, 10).await;
                    let _ = self.runtime.remove_container(cid, true).await;
                }
            }
            self.pods.remove(pod_id);
        }

        if let Some(d) = self.deployments.get_mut(&deployment_id) {
            d.status = DeploymentStatus::Stopped;
        }

        Ok(())
    }

    async fn handle_remove_deployment(&mut self, project: &str, name: &str) -> Result<()> {
        self.handle_stop(project, name).await?;
        let id = self
            .find_deployment_id(project, name)
            .ok_or_else(|| NexaError::DeploymentNotFound(format!("{project}/{name}")))?;
        self.deployments.remove(&id);
        Ok(())
    }

    async fn handle_scale(&mut self, project: &str, name: &str, replicas: u32) -> Result<Deployment> {
        let deployment_id = self
            .find_deployment_id(project, name)
            .ok_or_else(|| NexaError::DeploymentNotFound(format!("{project}/{name}")))?;

        if let Some(d) = self.deployments.get_mut(&deployment_id) {
            d.spec.replicas = replicas;
            d.updated_at = chrono::Utc::now();
        }

        self.reconcile_deployment(deployment_id).await?;
        Ok(self.deployments[&deployment_id].clone())
    }

    async fn handle_pod_logs(&self, project: &str, name: &str, tail: Option<u64>) -> Result<LogStream> {
        let pod = self
            .pods
            .values()
            .find(|p| p.project == project && p.deployment_name == name)
            .ok_or_else(|| NexaError::PodNotFound(format!("{project}/{name}")))?;

        let container_id = pod
            .container_id
            .as_ref()
            .ok_or_else(|| NexaError::Runtime("pod has no container".into()))?;

        self.runtime.logs(container_id, tail).await
    }

    async fn reconcile_deployment(&mut self, deployment_id: Uuid) -> Result<()> {
        let spec = self.deployments[&deployment_id].spec.clone();
        let desired = spec.replicas;

        let network_name = format!("nexa-{}", spec.project);
        let _ = self.runtime.create_network(&network_name).await;

        let mut current_pods: Vec<Uuid> = self
            .pods
            .values()
            .filter(|p| p.deployment_id == deployment_id)
            .map(|p| p.id)
            .collect();

        let current_count = current_pods.len() as u32;

        if current_count < desired {
            for i in current_count..desired {
                self.create_pod(deployment_id, &spec, i).await?;
            }
        } else if current_count > desired {
            current_pods.sort();
            for &pod_id in &current_pods[(desired as usize)..] {
                if let Some(pod) = self.pods.get(&pod_id) {
                    if let Some(cid) = &pod.container_id {
                        let _ = self.runtime.stop_container(cid, 10).await;
                        let _ = self.runtime.remove_container(cid, true).await;
                    }
                }
                self.pods.remove(&pod_id);
            }
        }

        let (all_running, any_failed) = self
            .pods
            .values()
            .filter(|p| p.deployment_id == deployment_id)
            .fold((true, false), |(all_r, any_f), p| {
                match p.status {
                    PodStatus::Running => (all_r, any_f),
                    PodStatus::Failed => (false, true),
                    _ => (false, any_f),
                }
            });

        if let Some(d) = self.deployments.get_mut(&deployment_id) {
            d.status = if all_running && desired > 0 {
                DeploymentStatus::Running
            } else if any_failed {
                DeploymentStatus::Degraded
            } else {
                DeploymentStatus::Pending
            };
        }

        Ok(())
    }

    async fn create_pod(&mut self, deployment_id: Uuid, spec: &DeploymentSpec, index: u32) -> Result<()> {
        let mut pod = Pod::new(
            deployment_id,
            &spec.project,
            &spec.deployment.name,
            index,
            &spec.image,
        );

        let container_name = pod.container_name();
        let network_name = format!("nexa-{}", spec.project);

        pod.status = PodStatus::Creating;

        let _ = self.runtime.pull_image(&spec.image).await;

        if self.runtime.container_exists(&container_name).await? {
            let _ = self.runtime.stop_container(&container_name, 5).await;
            let _ = self.runtime.remove_container(&container_name, true).await;
        }

        let ports: Vec<crate::ports::runtime::PortBinding> = spec
            .ports
            .iter()
            .map(|&p| crate::ports::runtime::PortBinding {
                container_port: p,
                host_port: if spec.replicas == 1 { Some(p) } else { None },
            })
            .collect();

        let mut labels = StdHashMap::new();
        labels.insert("managed-by".to_string(), "nexanet".to_string());
        labels.insert("nexa.project".to_string(), spec.project.clone());
        labels.insert("nexa.deployment".to_string(), spec.deployment.name.clone());
        labels.insert("nexa.pod-id".to_string(), pod.id.to_string());

        let config = ContainerConfig {
            name: container_name,
            image: spec.image.clone(),
            env: spec.env.clone(),
            ports,
            volumes: spec
                .volumes
                .iter()
                .map(|v| crate::ports::runtime::VolumeBinding {
                    source: v.source_name().to_string(),
                    target: v.mount_point().to_string(),
                    read_only: v.is_read_only(),
                })
                .collect(),
            labels,
            network: Some(network_name),
        };

        match self.runtime.create_container(&config).await {
            Ok(container_id) => {
                self.runtime.start_container(&container_id).await?;
                pod.container_id = Some(container_id);
                pod.status = PodStatus::Running;
            }
            Err(_) => {
                pod.status = PodStatus::Failed;
            }
        }

        self.pods.insert(pod.id, pod);
        Ok(())
    }

    fn find_deployment_id(&self, project: &str, name: &str) -> Option<Uuid> {
        self.deployments
            .values()
            .find(|d| d.project() == project && d.name() == name)
            .map(|d| d.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn handle_create_project_sends_command() {
        let (tx, mut rx) = mpsc::channel(16);
        let handle = OrchestratorHandle { tx };

        tokio::spawn(async move {
            if let Some(Command::CreateProject { name, reply }) = rx.recv().await {
                assert_eq!(name, "test-project");
                let _ = reply.send(Ok(Project::new("test-project")));
            }
        });

        let result = handle.create_project("test-project".into()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "test-project");
    }

    #[tokio::test]
    async fn handle_returns_error_when_orchestrator_stopped() {
        let (tx, rx) = mpsc::channel(1);
        let handle = OrchestratorHandle { tx };
        drop(rx);

        let result = handle.create_project("test".into()).await;
        assert!(result.is_err());
    }

    use std::collections::HashMap;
    use std::pin::Pin;
    use futures::Stream;
    use crate::ports::runtime::*;

    struct MockRuntime;

    #[async_trait::async_trait]
    impl ContainerRuntime for MockRuntime {
        async fn pull_image(&self, _image: &str) -> Result<()> { Ok(()) }
        async fn create_container(&self, config: &ContainerConfig) -> Result<String> {
            Ok(format!("mock-{}", config.name))
        }
        async fn start_container(&self, _id: &str) -> Result<()> { Ok(()) }
        async fn stop_container(&self, _id: &str, _timeout: u64) -> Result<()> { Ok(()) }
        async fn remove_container(&self, _id: &str, _force: bool) -> Result<()> { Ok(()) }
        async fn inspect_container(&self, _id: &str) -> Result<ContainerInfo> {
            Ok(ContainerInfo {
                id: "mock".into(),
                name: "mock".into(),
                image: "mock".into(),
                state: ContainerState::Running,
            })
        }
        async fn logs(&self, _id: &str, _tail: Option<u64>) -> Result<LogStream> {
            Ok(Box::pin(futures::stream::empty()))
        }
        async fn container_exists(&self, _name: &str) -> Result<bool> { Ok(false) }
        async fn create_network(&self, _name: &str) -> Result<String> { Ok("net-id".into()) }
        async fn remove_network(&self, _name: &str) -> Result<()> { Ok(()) }
        async fn connect_to_network(&self, _id: &str, _net: &str) -> Result<()> { Ok(()) }
    }

    fn spawn_test_orchestrator() -> OrchestratorHandle {
        Orchestrator::spawn(Arc::new(MockRuntime))
    }

    #[tokio::test]
    async fn deploy_creates_pods() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "test".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 2,
            image: "nginx:latest".into(),
            ports: vec![8080],
            env: HashMap::new(),
            volumes: vec![],
            secrets: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        let deployment = handle.deploy(spec).await.unwrap();
        assert_eq!(deployment.name(), "api");
        assert_eq!(deployment.project(), "test");

        let pods = handle.list_pods(Some("test".into())).await;
        assert_eq!(pods.len(), 2);
        assert!(pods.iter().all(|p| p.status == PodStatus::Running));
    }

    #[tokio::test]
    async fn list_projects_returns_auto_created() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "web".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            secrets: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        let projects = handle.list_projects().await;
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "myapp");
    }

    #[tokio::test]
    async fn scale_changes_pod_count() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "test".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            secrets: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        assert_eq!(handle.list_pods(None).await.len(), 1);

        handle.scale("test".into(), "api".into(), 3).await.unwrap();
        assert_eq!(handle.list_pods(None).await.len(), 3);

        handle.scale("test".into(), "api".into(), 1).await.unwrap();
        assert_eq!(handle.list_pods(None).await.len(), 1);
    }

    #[tokio::test]
    async fn stop_removes_pods() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "test".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 2,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            secrets: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        handle.stop("test".into(), "api".into()).await.unwrap();

        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 0);
    }
}
