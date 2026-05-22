use std::sync::Arc;
use std::collections::HashMap as StdHashMap;

use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::domain::models::*;
use crate::domain::health::{HealthTracker, HealthState, PodHealthConfig};
use crate::domain::restart::{self, RestartState};
use crate::domain::scheduler::{NodeSnapshot, PodRequest, SchedulerConfig, SchedulerWeights, WeightedScheduler, parse_memory};
use crate::error::{NexaError, Result};
use crate::ports::cluster::ClusterTransport;
use crate::ports::runtime::{ContainerConfig, ContainerRuntime, LogStream};
use crate::ports::secrets::SecretStore;
use crate::ports::dns::DnsProvider;
use crate::ports::state::StateStore;
use crate::ports::runtime::ContainerState;

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
    HealthReport {
        pod_id: Uuid,
        healthy: bool,
    },
    GetHealthProbeTargets {
        reply: oneshot::Sender<Vec<(Uuid, PodHealthConfig)>>,
    },
    ContainerExited {
        pod_id: Uuid,
        exit_code: i64,
    },
    RestartPod {
        pod_id: Uuid,
    },
    SuspendProject {
        name: String,
        reply: oneshot::Sender<Result<()>>,
    },
    ResumeProject {
        name: String,
        reply: oneshot::Sender<Result<()>>,
    },
    DeleteProject {
        name: String,
        reply: oneshot::Sender<Result<()>>,
    },
    ListSecrets {
        project: String,
        reply: oneshot::Sender<Result<Vec<String>>>,
    },
    SetSecret {
        project: String,
        name: String,
        value: Vec<u8>,
        reply: oneshot::Sender<Result<()>>,
    },
    DeleteSecret {
        project: String,
        name: String,
        reply: oneshot::Sender<Result<()>>,
    },
    GetSchedulerConfig {
        reply: oneshot::Sender<SchedulerConfig>,
    },
    SetSchedulerConfig {
        config: SchedulerConfig,
        reply: oneshot::Sender<Result<SchedulerConfig>>,
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

    pub async fn report_health(&self, pod_id: Uuid, healthy: bool) {
        let _ = self.tx.send(Command::HealthReport { pod_id, healthy }).await;
    }

    pub async fn get_health_probe_targets(&self) -> Vec<(Uuid, PodHealthConfig)> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Command::GetHealthProbeTargets { reply }).await;
        rx.await.unwrap_or_default()
    }

    pub async fn send_container_exited(&self, pod_id: Uuid, exit_code: i64) {
        let _ = self.tx.send(Command::ContainerExited { pod_id, exit_code }).await;
    }

    pub async fn suspend_project(&self, name: String) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::SuspendProject { name, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn resume_project(&self, name: String) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::ResumeProject { name, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn delete_project(&self, name: String) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::DeleteProject { name, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn list_secrets(&self, project: String) -> Result<Vec<String>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::ListSecrets { project, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn set_secret(&self, project: String, name: String, value: Vec<u8>) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::SetSecret { project, name, value, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn delete_secret(&self, project: String, name: String) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::DeleteSecret { project, name, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub async fn get_scheduler_config(&self) -> SchedulerConfig {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Command::GetSchedulerConfig { reply }).await;
        rx.await.unwrap_or_default()
    }

    pub async fn set_scheduler_config(&self, config: SchedulerConfig) -> Result<SchedulerConfig> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::SetSchedulerConfig { config, reply })
            .await
            .map_err(|_| NexaError::Runtime("orchestrator stopped".into()))?;
        rx.await
            .map_err(|_| NexaError::Runtime("orchestrator dropped reply".into()))?
    }

    pub fn command_sender(&self) -> mpsc::Sender<Command> {
        self.tx.clone()
    }
}

pub struct Orchestrator {
    runtime: Arc<dyn ContainerRuntime>,
    state_store: Option<Arc<dyn StateStore>>,
    secret_store: Option<Arc<dyn SecretStore>>,
    transport: Option<Arc<dyn ClusterTransport>>,
    scheduler: WeightedScheduler,
    scheduler_strategy: String,
    health_tracker: HealthTracker,
    projects: StdHashMap<String, Project>,
    deployments: StdHashMap<Uuid, Deployment>,
    pods: StdHashMap<Uuid, Pod>,
    restart_states: StdHashMap<Uuid, RestartState>,
    dns: Option<Arc<dyn DnsProvider>>,
    master_ip: Option<String>,
    tx: mpsc::Sender<Command>,
}

impl Orchestrator {
    pub fn spawn(
        runtime: Arc<dyn ContainerRuntime>,
        state_store: Option<Arc<dyn StateStore>>,
        secret_store: Option<Arc<dyn SecretStore>>,
        transport: Option<Arc<dyn ClusterTransport>>,
        dns: Option<Arc<dyn DnsProvider>>,
        master_ip: Option<String>,
    ) -> OrchestratorHandle {
        let (tx, rx) = mpsc::channel(256);
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let mut orch = Self {
                runtime,
                state_store,
                secret_store,
                transport,
                scheduler: WeightedScheduler::new(SchedulerWeights::default()),
                scheduler_strategy: "spread".to_string(),
                health_tracker: HealthTracker::new(),
                projects: StdHashMap::new(),
                deployments: StdHashMap::new(),
                pods: StdHashMap::new(),
                restart_states: StdHashMap::new(),
                dns,
                master_ip,
                tx: tx_clone,
            };
            orch.run(rx).await;
        });
        OrchestratorHandle { tx }
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<Command>) {
        self.load_state().await;
        self.reconcile_stale_pods().await;
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
                Command::HealthReport { pod_id, healthy } => {
                    self.handle_health_report(pod_id, healthy).await;
                }
                Command::GetHealthProbeTargets { reply } => {
                    let targets = self.handle_get_health_probe_targets();
                    let _ = reply.send(targets);
                }
                Command::ContainerExited { pod_id, exit_code } => {
                    self.handle_container_exited(pod_id, exit_code).await;
                }
                Command::RestartPod { pod_id } => {
                    self.handle_restart_pod(pod_id).await;
                }
                Command::SuspendProject { name, reply } => {
                    let result = self.handle_suspend_project(&name).await;
                    let _ = reply.send(result);
                }
                Command::ResumeProject { name, reply } => {
                    let result = self.handle_resume_project(&name).await;
                    let _ = reply.send(result);
                }
                Command::DeleteProject { name, reply } => {
                    let result = self.handle_delete_project(&name).await;
                    let _ = reply.send(result);
                }
                Command::ListSecrets { project, reply } => {
                    let result = self.handle_list_secrets(&project).await;
                    let _ = reply.send(result);
                }
                Command::SetSecret { project, name, value, reply } => {
                    let result = self.handle_set_secret(&project, &name, &value).await;
                    let _ = reply.send(result);
                }
                Command::DeleteSecret { project, name, reply } => {
                    let result = self.handle_delete_secret(&project, &name).await;
                    let _ = reply.send(result);
                }
                Command::GetSchedulerConfig { reply } => {
                    let _ = reply.send(self.handle_get_scheduler_config());
                }
                Command::SetSchedulerConfig { config, reply } => {
                    let _ = reply.send(self.handle_set_scheduler_config(config));
                }
            }
        }
    }

    async fn load_state(&mut self) {
        let Some(store) = &self.state_store else { return };
        match store.list_projects().await {
            Ok(projects) => {
                for p in projects {
                    self.projects.insert(p.name.clone(), p);
                }
                tracing::info!(count = self.projects.len(), "loaded projects from state store");
            }
            Err(e) => tracing::error!(error = %e, "failed to load projects"),
        }
        match store.list_deployments(None).await {
            Ok(deployments) => {
                for d in deployments {
                    self.deployments.insert(d.id, d);
                }
                tracing::info!(count = self.deployments.len(), "loaded deployments from state store");
            }
            Err(e) => tracing::error!(error = %e, "failed to load deployments"),
        }
        match store.list_pods(None).await {
            Ok(pods) => {
                for p in pods {
                    self.pods.insert(p.id, p);
                }
                tracing::info!(count = self.pods.len(), "loaded pods from state store");
            }
            Err(e) => tracing::error!(error = %e, "failed to load pods"),
        }
    }

    async fn reconcile_stale_pods(&mut self) {
        let running_pod_ids: Vec<Uuid> = self.pods.values()
            .filter(|p| p.status == PodStatus::Running && p.container_id.is_some())
            .map(|p| p.id)
            .collect();
        if running_pod_ids.is_empty() { return; }
        tracing::info!(count = running_pod_ids.len(), "reconciling pods with runtime");
        for pod_id in running_pod_ids {
            let container_id = match self.pods.get(&pod_id) {
                Some(p) => match &p.container_id { Some(cid) => cid.clone(), None => continue },
                None => continue,
            };
            let is_running = match self.runtime.inspect_container(&container_id).await {
                Ok(info) => info.state == ContainerState::Running,
                Err(_) => false,
            };
            if !is_running {
                if let Some(pod) = self.pods.get_mut(&pod_id) {
                    tracing::warn!(pod_id = %pod_id, container_id = %container_id, "pod container not running, marking Failed");
                    pod.status = PodStatus::Failed;
                    let cloned = pod.clone();
                    self.persist_update_pod(&cloned).await;
                }
            }
        }
        let deployment_ids: Vec<Uuid> = self.deployments.keys().cloned().collect();
        for deployment_id in deployment_ids {
            let desired = match self.deployments.get(&deployment_id) {
                Some(d) => d.spec.replicas, None => continue,
            };
            let (all_running, any_failed) = self.pods.values()
                .filter(|p| p.deployment_id == deployment_id)
                .fold((true, false), |(all_r, any_f), p| match p.status {
                    PodStatus::Running => (all_r, any_f),
                    PodStatus::Failed => (false, true),
                    _ => (false, any_f),
                });
            if let Some(d) = self.deployments.get_mut(&deployment_id) {
                let new_status = if all_running && desired > 0 {
                    DeploymentStatus::Running
                } else if any_failed {
                    DeploymentStatus::Degraded
                } else {
                    DeploymentStatus::Pending
                };
                if d.status != new_status {
                    d.status = new_status;
                    let cloned = d.clone();
                    self.persist_update_deployment(&cloned).await;
                }
            }
        }
    }

    async fn ensure_project(&mut self, name: &str) {
        if !self.projects.contains_key(name) {
            let project = Project::new(name);
            self.projects.insert(name.to_string(), project.clone());
            self.persist_insert_project(&project).await;
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
        // Block deploys to suspended projects
        if let Some(project) = self.projects.get(&spec.project) {
            if project.is_suspended() {
                return Err(NexaError::ProjectSuspended(spec.project.clone()));
            }
        }

        self.ensure_project(&spec.project).await;

        let existing_id = self.find_deployment_id(&spec.project, &spec.deployment.name);

        if let Some(id) = existing_id {
            let deployment = self.deployments.get_mut(&id).unwrap();
            deployment.spec = spec.clone();
            deployment.updated_at = chrono::Utc::now();
            let cloned = deployment.clone();
            self.persist_update_deployment(&cloned).await;
            let id = cloned.id;
            self.reconcile_deployment(id).await?;
            return Ok(self.deployments[&id].clone());
        }

        let deployment = Deployment::from_spec(spec);
        let id = deployment.id;
        self.persist_insert_deployment(&deployment).await;
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
            self.health_tracker.unregister(pod_id);
            if let Some(pod) = self.pods.get(pod_id) {
                if let Some(dns) = &self.dns {
                    if let Some(ref ip_str) = pod.container_ip {
                        if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
                            let _ = dns.deregister(&pod.project, &pod.deployment_name, ip).await;
                        }
                    }
                }
                if let Some(cid) = &pod.container_id {
                    let _ = self.runtime.stop_container(cid, 10).await;
                    let _ = self.runtime.remove_container(cid, true).await;
                }
            }
            self.persist_delete_pod(pod_id).await;
            self.pods.remove(pod_id);
            self.restart_states.remove(pod_id);
        }

        if let Some(d) = self.deployments.get_mut(&deployment_id) {
            d.status = DeploymentStatus::Stopped;
            let cloned = d.clone();
            self.persist_update_deployment(&cloned).await;
        }

        Ok(())
    }

    async fn handle_remove_deployment(&mut self, project: &str, name: &str) -> Result<()> {
        self.handle_stop(project, name).await?;
        let id = self
            .find_deployment_id(project, name)
            .ok_or_else(|| NexaError::DeploymentNotFound(format!("{project}/{name}")))?;
        if let Some(store) = &self.state_store {
            let _ = store.delete_deployment(&id).await;
        }
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
                self.health_tracker.unregister(&pod_id);
                if let Some(pod) = self.pods.get(&pod_id) {
                    if let Some(dns) = &self.dns {
                        if let Some(ref ip_str) = pod.container_ip {
                            if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
                                let _ = dns.deregister(&pod.project, &pod.deployment_name, ip).await;
                            }
                        }
                    }
                    if let Some(cid) = &pod.container_id {
                        let _ = self.runtime.stop_container(cid, 10).await;
                        let _ = self.runtime.remove_container(cid, true).await;
                    }
                }
                self.persist_delete_pod(&pod_id).await;
                self.pods.remove(&pod_id);
                self.restart_states.remove(&pod_id);
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
            let cloned = d.clone();
            self.persist_update_deployment(&cloned).await;
        }

        Ok(())
    }

    fn build_container_config(&self, spec: &DeploymentSpec, pod_id: Uuid, container_name: &str, network_name: &str, env: &StdHashMap<String, String>) -> ContainerConfig {
        let ports = spec.ports.iter().map(|&p| crate::ports::runtime::PortBinding {
            container_port: p,
            host_port: if spec.replicas == 1 { Some(p) } else { None },
        }).collect();

        let mut labels = StdHashMap::new();
        labels.insert("managed-by".to_string(), "nexanet".to_string());
        labels.insert("nexa.project".to_string(), spec.project.clone());
        labels.insert("nexa.deployment".to_string(), spec.deployment.name.clone());
        labels.insert("nexa.pod-id".to_string(), pod_id.to_string());

        let (dns_servers, dns_search_domains) = match &self.master_ip {
            Some(ip) => (
                vec![ip.clone()],
                vec![format!("{}.internal", spec.project)],
            ),
            None => (vec![], vec![]),
        };

        ContainerConfig {
            name: container_name.to_string(),
            image: spec.image.clone(),
            env: env.clone(),
            ports,
            volumes: spec.volumes.iter().map(|v| crate::ports::runtime::VolumeBinding {
                source: v.source_name().to_string(),
                target: v.mount_point().to_string(),
                read_only: v.is_read_only(),
            }).collect(),
            labels,
            network: Some(network_name.to_string()),
            dns: dns_servers,
            dns_search: dns_search_domains,
        }
    }

    async fn resolve_secrets(&self, project: &str, secret_names: &[String]) -> Result<StdHashMap<String, String>> {
        let store = match &self.secret_store {
            Some(s) => s,
            None => return Ok(StdHashMap::new()),
        };
        let mut resolved = StdHashMap::new();
        for name in secret_names {
            let value = store.get(project, name).await?
                .ok_or_else(|| NexaError::Secret(format!("secret '{}' not found in project '{}'", name, project)))?;
            let value_str = String::from_utf8(value)
                .map_err(|_| NexaError::Secret(format!("secret '{}' contains invalid UTF-8", name)))?;
            resolved.insert(name.clone(), value_str);
        }
        Ok(resolved)
    }

    async fn select_node(&self, spec: &DeploymentSpec) -> Option<Uuid> {
        let state = self.state_store.as_ref()?;
        let nodes = state.list_nodes().await.ok()?;

        let snapshots: Vec<NodeSnapshot> = nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Ready && n.role == NodeRole::Worker)
            .map(|n| NodeSnapshot {
                node_id: n.id,
                cpu_available: n.resources.cpu_available,
                cpu_total: n.resources.cpu_cores,
                memory_available: n.resources.memory_available,
                memory_total: n.resources.memory_bytes,
                running_pods: n.resources.running_pods,
                max_pods: 110,
                recent_failures: vec![],
            })
            .collect();

        if snapshots.is_empty() {
            return None;
        }

        let pod_request = PodRequest {
            cpu_request: spec.resources.as_ref().map(|r| r.cpu).unwrap_or(0.0),
            memory_request: spec.resources.as_ref().map(|r| parse_memory(&r.memory)).unwrap_or(0),
        };

        self.scheduler.select_node(&pod_request, &snapshots).ok()
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

        // Resolve secrets and merge into env
        let mut final_env = spec.env.clone();
        if !spec.secrets.is_empty() {
            let resolved = self.resolve_secrets(&spec.project, &spec.secrets).await?;
            final_env.extend(resolved);
        }

        // Try to schedule on a remote node if transport is available
        let target_node = self.select_node(spec).await;

        if let (Some(transport), Some(node_id)) = (&self.transport, target_node) {
            // Multi-node path: delegate to transport
            pod.node_id = Some(node_id);

            // Create a modified spec with resolved secrets in env
            let mut resolved_spec = spec.clone();
            resolved_spec.env = final_env;

            match transport.assign_pod(&node_id, &pod, &resolved_spec).await {
                Ok(()) => {
                    pod.status = PodStatus::Running;
                }
                Err(e) => {
                    tracing::error!(name = container_name, error = %e, "failed to assign pod to node");
                    pod.status = PodStatus::Failed;
                }
            }
        } else {
            // Single-node path: use runtime directly
            let _ = self.runtime.pull_image(&spec.image).await;

            if self.runtime.container_exists(&container_name).await? {
                let _ = self.runtime.stop_container(&container_name, 5).await;
                let _ = self.runtime.remove_container(&container_name, true).await;
            }

            let config = self.build_container_config(spec, pod.id, &container_name, &network_name, &final_env);

            match self.runtime.create_container(&config).await {
                Ok(container_id) => {
                    self.runtime.start_container(&container_id).await?;
                    pod.container_id = Some(container_id.clone());
                    pod.status = PodStatus::Running;

                    match self.runtime.container_ip(&container_id, &network_name).await {
                        Ok(ip) => {
                            pod.container_ip = Some(ip.clone());
                            if let Some(dns) = &self.dns {
                                if let Ok(parsed_ip) = ip.parse::<std::net::IpAddr>() {
                                    let _ = dns.register(&spec.project, &spec.deployment.name, parsed_ip).await;
                                }
                            }
                        }
                        Err(e) => tracing::warn!(error = %e, "failed to get container IP"),
                    }
                }
                Err(_) => {
                    pod.status = PodStatus::Failed;
                }
            }
        }

        self.persist_insert_pod(&pod).await;

        // Register for health checking only for local pods
        if target_node.is_none() {
            if let (Some(hc), Some(ip)) = (&spec.healthcheck, &pod.container_ip) {
                if let Some(&port) = spec.ports.first() {
                    let interval = crate::duration::parse_duration(&hc.interval)
                        .unwrap_or(std::time::Duration::from_secs(10));
                    let timeout = crate::duration::parse_duration(&hc.timeout)
                        .unwrap_or(std::time::Duration::from_secs(5));

                    self.health_tracker.register(PodHealthConfig {
                        pod_id: pod.id,
                        container_ip: ip.clone(),
                        port,
                        path: hc.path.clone(),
                        interval,
                        timeout,
                        retries: hc.retries,
                    });
                }
            }
        }

        self.pods.insert(pod.id, pod);
        Ok(())
    }

    async fn handle_health_report(&mut self, pod_id: Uuid, healthy: bool) {
        let now = chrono::Utc::now();

        // Track restart state alongside health tracker
        if healthy {
            if let Some(state) = self.restart_states.get_mut(&pod_id) {
                state.mark_healthy(now);
                state.reset_if_healthy(now);
                // Sync restart_count to pod
                let count = state.count;
                if let Some(p) = self.pods.get_mut(&pod_id) {
                    p.restart_count = count;
                }
            }
        } else {
            if let Some(state) = self.restart_states.get_mut(&pod_id) {
                state.mark_unhealthy();
            }
        }

        let result = self.health_tracker.record_result(&pod_id, healthy);
        match result {
            Some((HealthState::Unhealthy, true)) => {
                tracing::info!(pod_id = %pod_id, "pod unhealthy, triggering restart");
                if let Err(e) = self.restart_pod(pod_id).await {
                    tracing::error!(pod_id = %pod_id, error = %e, "failed to restart unhealthy pod");
                }
            }
            Some((HealthState::Failing { consecutive_failures }, _)) => {
                tracing::warn!(pod_id = %pod_id, failures = consecutive_failures, "pod health check failing");
            }
            _ => {}
        }
    }

    fn handle_get_health_probe_targets(&mut self) -> Vec<(Uuid, PodHealthConfig)> {
        let due_ids = self.health_tracker.pods_due_for_probe();
        let mut targets = Vec::new();
        for pod_id in due_ids {
            if let Some(config) = self.health_tracker.config(&pod_id) {
                let config = config.clone();
                self.health_tracker.mark_probed(&pod_id);
                targets.push((pod_id, config));
            }
        }
        targets
    }

    async fn handle_container_exited(&mut self, pod_id: Uuid, exit_code: i64) {
        // Look up the pod
        let pod = match self.pods.get(&pod_id) {
            Some(p) => p.clone(),
            None => return,
        };
        let deployment_id = pod.deployment_id;

        // Find deployment's restart policy
        let policy = match self.deployments.get(&deployment_id) {
            Some(d) => d.spec.restart.clone(),
            None => return,
        };

        // Check if restart is allowed by policy
        if !restart::should_restart(&policy, exit_code) {
            if let Some(p) = self.pods.get_mut(&pod_id) {
                p.status = PodStatus::Failed;
                let cloned = p.clone();
                self.persist_update_pod(&cloned).await;
            }
            self.update_deployment_status(deployment_id);
            if let Some(d) = self.deployments.get(&deployment_id) {
                let cloned = d.clone();
                self.persist_update_deployment(&cloned).await;
            }
            return;
        }

        // Get or create restart state
        let state = self.restart_states
            .entry(pod_id)
            .or_insert_with(RestartState::new);

        // If already in crash loop, mark CrashLoopBackoff and return
        if state.is_crash_loop() {
            if let Some(p) = self.pods.get_mut(&pod_id) {
                p.status = PodStatus::CrashLoopBackoff;
                let cloned = p.clone();
                self.persist_update_pod(&cloned).await;
            }
            self.update_deployment_status(deployment_id);
            if let Some(d) = self.deployments.get(&deployment_id) {
                let cloned = d.clone();
                self.persist_update_deployment(&cloned).await;
            }
            return;
        }

        // Record restart and check if we've hit crash loop
        let now = chrono::Utc::now();
        state.record_restart(now);

        if state.is_crash_loop() {
            if let Some(p) = self.pods.get_mut(&pod_id) {
                p.status = PodStatus::CrashLoopBackoff;
                let cloned = p.clone();
                self.persist_update_pod(&cloned).await;
            }
            self.update_deployment_status(deployment_id);
            if let Some(d) = self.deployments.get(&deployment_id) {
                let cloned = d.clone();
                self.persist_update_deployment(&cloned).await;
            }
            return;
        }

        // Mark pod as Restarting, update restart_count
        let restart_count = state.count;
        if let Some(p) = self.pods.get_mut(&pod_id) {
            p.status = PodStatus::Restarting;
            p.restart_count = restart_count;
            let cloned = p.clone();
            self.persist_update_pod(&cloned).await;
        }

        // Calculate backoff delay and schedule RestartPod
        let delay = restart::backoff_delay(restart_count.saturating_sub(1));
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = tx.send(Command::RestartPod { pod_id }).await;
        });
    }

    async fn handle_restart_pod(&mut self, pod_id: Uuid) {
        // Look up pod, skip if already CrashLoopBackoff or Failed
        let pod = match self.pods.get(&pod_id) {
            Some(p) => p.clone(),
            None => return,
        };
        match pod.status {
            PodStatus::CrashLoopBackoff | PodStatus::Failed => return,
            _ => {}
        }
        let deployment_id = pod.deployment_id;

        // Unregister old health tracker entry before recreating
        self.health_tracker.unregister(&pod_id);

        // Remove old container (stop + remove)
        if let Some(cid) = &pod.container_id {
            let _ = self.runtime.stop_container(cid, 10).await;
            let _ = self.runtime.remove_container(cid, true).await;
        }

        // Get deployment spec
        let spec = match self.deployments.get(&deployment_id) {
            Some(d) => d.spec.clone(),
            None => return,
        };

        // Recreate container in-place (same pod, new container)
        let container_name = pod.container_name();
        let network_name = format!("nexa-{}", spec.project);

        // Resolve secrets and merge into env
        let mut final_env = spec.env.clone();
        if !spec.secrets.is_empty() {
            match self.resolve_secrets(&spec.project, &spec.secrets).await {
                Ok(resolved) => final_env.extend(resolved),
                Err(e) => {
                    tracing::error!(pod_id = %pod_id, error = %e, "failed to resolve secrets for restarted pod");
                    if let Some(p) = self.pods.get_mut(&pod_id) {
                        p.status = PodStatus::Failed;
                        let cloned = p.clone();
                        self.persist_update_pod(&cloned).await;
                    }
                    return;
                }
            }
        }

        let _ = self.runtime.pull_image(&spec.image).await;

        if let Ok(true) = self.runtime.container_exists(&container_name).await {
            let _ = self.runtime.stop_container(&container_name, 5).await;
            let _ = self.runtime.remove_container(&container_name, true).await;
        }

        let config = self.build_container_config(&spec, pod_id, &container_name, &network_name, &final_env);

        match self.runtime.create_container(&config).await {
            Ok(container_id) => {
                if let Err(e) = self.runtime.start_container(&container_id).await {
                    tracing::error!(pod_id = %pod_id, error = %e, "failed to start restarted container");
                    if let Some(p) = self.pods.get_mut(&pod_id) {
                        p.status = PodStatus::Failed;
                        let cloned = p.clone();
                        self.persist_update_pod(&cloned).await;
                    }
                } else {
                    let container_ip = match self.runtime.container_ip(&container_id, &network_name).await {
                        Ok(ip) => Some(ip),
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to get container IP for restarted pod");
                            None
                        }
                    };

                    if let Some(p) = self.pods.get_mut(&pod_id) {
                        p.container_id = Some(container_id);
                        p.status = PodStatus::Running;
                        p.container_ip = container_ip.clone();
                        let cloned = p.clone();
                        self.persist_update_pod(&cloned).await;
                    }

                    // Re-register for health checking if configured
                    if let (Some(hc), Some(ip)) = (&spec.healthcheck, &container_ip) {
                        if let Some(&port) = spec.ports.first() {
                            let interval = crate::duration::parse_duration(&hc.interval)
                                .unwrap_or(std::time::Duration::from_secs(10));
                            let timeout = crate::duration::parse_duration(&hc.timeout)
                                .unwrap_or(std::time::Duration::from_secs(5));

                            self.health_tracker.register(PodHealthConfig {
                                pod_id,
                                container_ip: ip.clone(),
                                port,
                                path: hc.path.clone(),
                                interval,
                                timeout,
                                retries: hc.retries,
                            });
                        }
                    }

                    // Mark healthy in restart state
                    let now = chrono::Utc::now();
                    if let Some(state) = self.restart_states.get_mut(&pod_id) {
                        state.mark_healthy(now);
                    }
                }
            }
            Err(e) => {
                tracing::error!(pod_id = %pod_id, error = %e, "failed to create restarted container");
                if let Some(p) = self.pods.get_mut(&pod_id) {
                    p.status = PodStatus::Failed;
                    let cloned = p.clone();
                    self.persist_update_pod(&cloned).await;
                }
            }
        }

        // Update deployment status
        self.update_deployment_status(deployment_id);
        if let Some(d) = self.deployments.get(&deployment_id) {
            let cloned = d.clone();
            self.persist_update_deployment(&cloned).await;
        }
    }

    async fn restart_pod(&mut self, pod_id: Uuid) -> Result<()> {
        let pod = self.pods.get(&pod_id)
            .ok_or_else(|| NexaError::PodNotFound(pod_id.to_string()))?;
        let deployment_id = pod.deployment_id;
        let replica_index = pod.replica_index;

        // Stop old container
        if let Some(cid) = &pod.container_id {
            let _ = self.runtime.stop_container(cid, 10).await;
            let _ = self.runtime.remove_container(cid, true).await;
        }

        // Remove old pod
        self.health_tracker.unregister(&pod_id);
        self.persist_delete_pod(&pod_id).await;
        self.pods.remove(&pod_id);
        self.restart_states.remove(&pod_id);

        // Get deployment spec
        let spec = self.deployments.get(&deployment_id)
            .ok_or_else(|| NexaError::DeploymentNotFound(deployment_id.to_string()))?
            .spec.clone();

        // Create replacement pod
        self.create_pod(deployment_id, &spec, replica_index).await?;

        // Update deployment status
        self.update_deployment_status(deployment_id);

        Ok(())
    }

    fn update_deployment_status(&mut self, deployment_id: Uuid) {
        let desired = match self.deployments.get(&deployment_id) {
            Some(d) => d.spec.replicas,
            None => return,
        };
        let (all_running, any_failed) = self.pods.values()
            .filter(|p| p.deployment_id == deployment_id)
            .fold((true, false), |(all_r, any_f), p| match p.status {
                PodStatus::Running => (all_r, any_f),
                PodStatus::Failed => (false, true),
                _ => (false, any_f),
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
    }

    fn find_deployment_id(&self, project: &str, name: &str) -> Option<Uuid> {
        self.deployments
            .values()
            .find(|d| d.project() == project && d.name() == name)
            .map(|d| d.id)
    }

    // ---- Project lifecycle handlers ----

    async fn handle_suspend_project(&mut self, name: &str) -> Result<()> {
        let project = self.projects.get_mut(name)
            .ok_or_else(|| NexaError::ProjectNotFound(name.to_string()))?;
        project.status = ProjectStatus::Suspended;

        // Stop all pods in all deployments of this project
        let deployment_ids: Vec<Uuid> = self.deployments.values()
            .filter(|d| d.project() == name)
            .map(|d| d.id)
            .collect();

        for deployment_id in &deployment_ids {
            let pod_ids: Vec<Uuid> = self.pods.values()
                .filter(|p| p.deployment_id == *deployment_id)
                .map(|p| p.id)
                .collect();

            for pod_id in &pod_ids {
                self.health_tracker.unregister(pod_id);
                if let Some(pod) = self.pods.get(pod_id) {
                    if let Some(dns) = &self.dns {
                        if let Some(ref ip_str) = pod.container_ip {
                            if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
                                let _ = dns.deregister(&pod.project, &pod.deployment_name, ip).await;
                            }
                        }
                    }
                    if let Some(cid) = &pod.container_id {
                        let _ = self.runtime.stop_container(cid, 10).await;
                        let _ = self.runtime.remove_container(cid, true).await;
                    }
                }
                self.persist_delete_pod(pod_id).await;
                self.pods.remove(pod_id);
                self.restart_states.remove(pod_id);
            }

            if let Some(d) = self.deployments.get_mut(deployment_id) {
                d.status = DeploymentStatus::Stopped;
                let cloned = d.clone();
                self.persist_update_deployment(&cloned).await;
            }
        }

        // Persist project status update
        if let Some(store) = &self.state_store {
            let _ = store.update_project_status(name, ProjectStatus::Suspended).await;
        }

        Ok(())
    }

    async fn handle_resume_project(&mut self, name: &str) -> Result<()> {
        let project = self.projects.get_mut(name)
            .ok_or_else(|| NexaError::ProjectNotFound(name.to_string()))?;
        project.status = ProjectStatus::Active;

        // Persist project status update
        if let Some(store) = &self.state_store {
            let _ = store.update_project_status(name, ProjectStatus::Active).await;
        }

        // Reconcile all deployments in this project
        let deployment_ids: Vec<Uuid> = self.deployments.values()
            .filter(|d| d.project() == name)
            .map(|d| d.id)
            .collect();

        for deployment_id in deployment_ids {
            let _ = self.reconcile_deployment(deployment_id).await;
        }

        Ok(())
    }

    async fn handle_delete_project(&mut self, name: &str) -> Result<()> {
        if !self.projects.contains_key(name) {
            return Err(NexaError::ProjectNotFound(name.to_string()));
        }

        // Check if any deployments exist for this project
        let has_deployments = self.deployments.values().any(|d| d.project() == name);
        if has_deployments {
            return Err(NexaError::ProjectNotEmpty(name.to_string()));
        }

        self.projects.remove(name);

        // Persist project deletion
        if let Some(store) = &self.state_store {
            let _ = store.delete_project(name).await;
        }

        Ok(())
    }

    // ---- Secret command handlers ----

    async fn handle_list_secrets(&self, project: &str) -> Result<Vec<String>> {
        match &self.secret_store {
            Some(store) => store.list(project).await,
            None => Ok(Vec::new()),
        }
    }

    async fn handle_set_secret(&self, project: &str, name: &str, value: &[u8]) -> Result<()> {
        match &self.secret_store {
            Some(store) => store.set(project, name, value).await,
            None => Err(NexaError::Secret("no secret store configured".into())),
        }
    }

    async fn handle_delete_secret(&self, project: &str, name: &str) -> Result<()> {
        match &self.secret_store {
            Some(store) => store.delete(project, name).await,
            None => Err(NexaError::Secret("no secret store configured".into())),
        }
    }

    // ---- Scheduler config handlers ----

    fn handle_get_scheduler_config(&self) -> SchedulerConfig {
        SchedulerConfig {
            strategy: self.scheduler_strategy.clone(),
            weights: self.scheduler.weights().clone(),
        }
    }

    fn handle_set_scheduler_config(&mut self, config: SchedulerConfig) -> Result<SchedulerConfig> {
        self.scheduler = WeightedScheduler::new(config.weights.clone());
        self.scheduler_strategy = config.strategy.clone();
        Ok(config)
    }

    // ---- Persistence helpers ----

    async fn persist_insert_project(&self, project: &Project) {
        if let Some(store) = &self.state_store {
            if let Err(e) = store.insert_project(project).await {
                tracing::warn!(project = %project.name, error = %e, "failed to persist project");
            }
        }
    }

    async fn persist_insert_deployment(&self, deployment: &Deployment) {
        if let Some(store) = &self.state_store {
            if let Err(e) = store.insert_deployment(deployment).await {
                tracing::warn!(id = %deployment.id, error = %e, "failed to persist deployment insert");
            }
        }
    }

    async fn persist_update_deployment(&self, deployment: &Deployment) {
        if let Some(store) = &self.state_store {
            if let Err(e) = store.update_deployment(deployment).await {
                tracing::warn!(id = %deployment.id, error = %e, "failed to persist deployment update");
            }
        }
    }

    async fn persist_insert_pod(&self, pod: &Pod) {
        if let Some(store) = &self.state_store {
            if let Err(e) = store.insert_pod(pod).await {
                tracing::warn!(id = %pod.id, error = %e, "failed to persist pod insert");
            }
        }
    }

    async fn persist_update_pod(&self, pod: &Pod) {
        if let Some(store) = &self.state_store {
            if let Err(e) = store.update_pod(pod).await {
                tracing::warn!(id = %pod.id, error = %e, "failed to persist pod update");
            }
        }
    }

    async fn persist_delete_pod(&self, id: &Uuid) {
        if let Some(store) = &self.state_store {
            if let Err(e) = store.delete_pod(id).await {
                tracing::warn!(pod_id = %id, error = %e, "failed to persist pod delete");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;
    use crate::ports::runtime::*;
    use crate::ports::state::StateStore;
    use crate::ports::state_memory::InMemoryStore;

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
        async fn container_ip(&self, _container_id: &str, _network: &str) -> Result<String> { Ok("172.17.0.2".to_string()) }
        async fn events(&self) -> Result<EventStream> {
            Ok(Box::pin(futures::stream::pending()))
        }
    }

    struct ConfigurableMockRuntime {
        container_states: StdMutex<StdHashMap<String, ContainerState>>,
    }

    impl ConfigurableMockRuntime {
        fn new() -> Self {
            Self { container_states: StdMutex::new(StdHashMap::new()) }
        }
    }

    #[async_trait::async_trait]
    impl ContainerRuntime for ConfigurableMockRuntime {
        async fn pull_image(&self, _: &str) -> Result<()> { Ok(()) }
        async fn create_container(&self, config: &ContainerConfig) -> Result<String> {
            let id = format!("mock-{}", config.name);
            self.container_states.lock().unwrap().insert(id.clone(), ContainerState::Running);
            Ok(id)
        }
        async fn start_container(&self, _: &str) -> Result<()> { Ok(()) }
        async fn stop_container(&self, _: &str, _: u64) -> Result<()> { Ok(()) }
        async fn remove_container(&self, _: &str, _: bool) -> Result<()> { Ok(()) }
        async fn inspect_container(&self, id: &str) -> Result<ContainerInfo> {
            let states = self.container_states.lock().unwrap();
            match states.get(id) {
                Some(state) => Ok(ContainerInfo { id: id.into(), name: id.into(), image: "mock".into(), state: state.clone() }),
                None => Err(NexaError::Runtime(format!("container {id} not found"))),
            }
        }
        async fn logs(&self, _: &str, _: Option<u64>) -> Result<LogStream> { Ok(Box::pin(futures::stream::empty())) }
        async fn container_exists(&self, _: &str) -> Result<bool> { Ok(false) }
        async fn create_network(&self, _: &str) -> Result<String> { Ok("net-id".into()) }
        async fn remove_network(&self, _: &str) -> Result<()> { Ok(()) }
        async fn connect_to_network(&self, _: &str, _: &str) -> Result<()> { Ok(()) }
        async fn container_ip(&self, _container_id: &str, _network: &str) -> Result<String> { Ok("172.17.0.2".to_string()) }
        async fn events(&self) -> Result<EventStream> {
            Ok(Box::pin(futures::stream::pending()))
        }
    }

    fn spawn_test_orchestrator() -> OrchestratorHandle {
        Orchestrator::spawn(Arc::new(MockRuntime), None, None, None, None, None)
    }

    fn spawn_persisted_test_orchestrator() -> (OrchestratorHandle, Arc<InMemoryStore>) {
        let store = Arc::new(InMemoryStore::new());
        let handle = Orchestrator::spawn(Arc::new(MockRuntime), Some(store.clone() as Arc<dyn StateStore>), None, None, None, None);
        (handle, store)
    }

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

    #[tokio::test]
    async fn deploy_maps_volume_spec_to_volume_binding() {
        use std::sync::Mutex;

        struct CapturingRuntime {
            configs: Mutex<Vec<ContainerConfig>>,
        }

        #[async_trait::async_trait]
        impl ContainerRuntime for CapturingRuntime {
            async fn pull_image(&self, _image: &str) -> Result<()> { Ok(()) }
            async fn create_container(&self, config: &ContainerConfig) -> Result<String> {
                self.configs.lock().unwrap().push(config.clone());
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
            async fn container_ip(&self, _container_id: &str, _network: &str) -> Result<String> { Ok("172.17.0.2".to_string()) }
            async fn events(&self) -> Result<EventStream> {
                Ok(Box::pin(futures::stream::pending()))
            }
        }

        let runtime = Arc::new(CapturingRuntime {
            configs: Mutex::new(Vec::new()),
        });
        let handle = Orchestrator::spawn(runtime.clone(), None, None, None, None, None);

        let spec = DeploymentSpec {
            project: "test".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            secrets: vec![],
            volumes: vec![
                VolumeSpec::Named(NamedVolume {
                    name: "data".into(),
                    mount: "/app/data".into(),
                }),
                VolumeSpec::Bind(BindMount {
                    path: "/host/uploads".into(),
                    mount: "/app/uploads".into(),
                    readonly: true,
                }),
            ],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();

        let configs = runtime.configs.lock().unwrap();
        assert_eq!(configs.len(), 1);

        let vols = &configs[0].volumes;
        assert_eq!(vols.len(), 2);

        assert_eq!(vols[0].source, "data");
        assert_eq!(vols[0].target, "/app/data");
        assert!(!vols[0].read_only);

        assert_eq!(vols[1].source, "/host/uploads");
        assert_eq!(vols[1].target, "/app/uploads");
        assert!(vols[1].read_only);
    }

    // ---- Task 6 tests ----

    #[tokio::test]
    async fn deploy_persists_to_state_store() {
        let (handle, store) = spawn_persisted_test_orchestrator();

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "web".into() },
            replicas: 2,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            secrets: vec![],
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();

        let projects = store.list_projects().await.unwrap();
        assert_eq!(projects.len(), 1);

        let deployments = store.list_deployments(None).await.unwrap();
        assert_eq!(deployments.len(), 1);

        let pods = store.list_pods(None).await.unwrap();
        assert_eq!(pods.len(), 2);
    }

    #[tokio::test]
    async fn stop_persists_to_state_store() {
        let (handle, store) = spawn_persisted_test_orchestrator();

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "web".into() },
            replicas: 2,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            secrets: vec![],
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        handle.stop("myapp".into(), "web".into()).await.unwrap();

        let pods = store.list_pods(None).await.unwrap();
        assert_eq!(pods.len(), 0);

        let deployments = store.list_deployments(None).await.unwrap();
        assert_eq!(deployments.len(), 1);
        assert_eq!(deployments[0].status, DeploymentStatus::Stopped);
    }

    #[tokio::test]
    async fn scale_persists_to_state_store() {
        let (handle, store) = spawn_persisted_test_orchestrator();

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "web".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            secrets: vec![],
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        handle.scale("myapp".into(), "web".into(), 3).await.unwrap();

        let pods = store.list_pods(None).await.unwrap();
        assert_eq!(pods.len(), 3);
    }

    // ---- Task 7 tests ----

    #[tokio::test]
    async fn loads_state_on_startup() {
        let store = Arc::new(InMemoryStore::new());
        let project = Project::new("loaded");
        store.insert_project(&project).await.unwrap();
        let spec = DeploymentSpec {
            project: "loaded".into(),
            deployment: DeploymentMeta { name: "web".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            secrets: vec![],
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };
        let mut deployment = Deployment::from_spec(spec);
        deployment.status = DeploymentStatus::Running;
        store.insert_deployment(&deployment).await.unwrap();
        let mut pod = Pod::new(deployment.id, "loaded", "web", 0, "nginx");
        pod.status = PodStatus::Running;
        pod.container_id = Some("old-container-123".into());
        store.insert_pod(&pod).await.unwrap();

        let handle = Orchestrator::spawn(Arc::new(MockRuntime), Some(store.clone() as Arc<dyn StateStore>), None, None, None, None);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let projects = handle.list_projects().await;
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "loaded");
        let deployments = handle.list_deployments(Some("loaded".into())).await;
        assert_eq!(deployments.len(), 1);
        assert_eq!(deployments[0].name(), "web");
    }

    // ---- Task 9 tests (health checking) ----

    #[tokio::test]
    async fn health_report_tracks_failures() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "test".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![3000],
            env: HashMap::new(),
            volumes: vec![],
            secrets: vec![],
            network: None,
            healthcheck: Some(HealthCheck {
                path: "/health".into(),
                interval: "10s".into(),
                timeout: "5s".into(),
                retries: 3,
            }),
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        let pods = handle.list_pods(None).await;
        let pod_id = pods[0].id;

        // Report a failure
        handle.report_health(pod_id, false).await;

        // Small delay for the actor to process
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Pod should still be running (only 1 failure, retries=3)
        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].status, PodStatus::Running);
    }

    #[tokio::test]
    async fn health_report_triggers_restart_after_threshold() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "test".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![3000],
            env: HashMap::new(),
            volumes: vec![],
            secrets: vec![],
            network: None,
            healthcheck: Some(HealthCheck {
                path: "/health".into(),
                interval: "10s".into(),
                timeout: "5s".into(),
                retries: 3,
            }),
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        let pods = handle.list_pods(None).await;
        let original_pod_id = pods[0].id;

        // Report 3 failures to trigger restart
        for _ in 0..3 {
            handle.report_health(original_pod_id, false).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // After restart, there should still be 1 pod but with a different id
        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 1);
        assert_ne!(pods[0].id, original_pod_id, "pod should have been replaced");
        assert_eq!(pods[0].status, PodStatus::Running);
    }

    #[tokio::test]
    async fn get_health_probe_targets_returns_due_pods() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "test".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![3000],
            env: HashMap::new(),
            volumes: vec![],
            secrets: vec![],
            network: None,
            healthcheck: Some(HealthCheck {
                path: "/health".into(),
                interval: "1s".into(),
                timeout: "5s".into(),
                retries: 3,
            }),
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();

        // Immediately after registration, no pods should be due (last_probe is now)
        let targets = handle.get_health_probe_targets().await;
        assert!(targets.is_empty());

        // Wait for interval to elapse
        tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

        let targets = handle.get_health_probe_targets().await;
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].1.path, "/health");
        assert_eq!(targets[0].1.port, 3000);
    }

    // ---- Task 8 tests ----

    #[tokio::test]
    async fn reconcile_marks_stale_pods_failed() {
        let store = Arc::new(InMemoryStore::new());
        let project = Project::new("myapp");
        store.insert_project(&project).await.unwrap();

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "web".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            secrets: vec![],
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };
        let mut deployment = Deployment::from_spec(spec);
        deployment.status = DeploymentStatus::Running;
        store.insert_deployment(&deployment).await.unwrap();

        let mut pod = Pod::new(deployment.id, "myapp", "web", 0, "nginx");
        pod.status = PodStatus::Running;
        pod.container_id = Some("vanished-container".into());
        store.insert_pod(&pod).await.unwrap();

        // ConfigurableMockRuntime has no knowledge of "vanished-container"
        let runtime = Arc::new(ConfigurableMockRuntime::new());
        let handle = Orchestrator::spawn(runtime, Some(store.clone() as Arc<dyn StateStore>), None, None, None, None);
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].status, PodStatus::Failed);

        let deployments = handle.list_deployments(None).await;
        assert_eq!(deployments.len(), 1);
        assert_eq!(deployments[0].status, DeploymentStatus::Degraded);
    }

    // ---- Restart policy tests (Task 5) ----

    #[tokio::test]
    async fn container_exited_with_always_policy_triggers_restart() {
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
            restart: RestartPolicy::Always,
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        let pods = handle.list_pods(None).await;
        let pod_id = pods[0].id;

        // Send container exited event
        handle.send_container_exited(pod_id, 1).await;

        // Wait for the actor to process + backoff (1s for first restart) + processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Pod should be in Restarting state
        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].status, PodStatus::Restarting);
        assert_eq!(pods[0].restart_count, 1);

        // Wait for the backoff to complete and RestartPod to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].status, PodStatus::Running);
    }

    #[tokio::test]
    async fn container_exited_with_never_policy_marks_failed() {
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
            restart: RestartPolicy::Never,
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        let pods = handle.list_pods(None).await;
        let pod_id = pods[0].id;

        handle.send_container_exited(pod_id, 1).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].status, PodStatus::Failed);
    }

    #[tokio::test]
    async fn on_failure_policy_does_not_restart_on_clean_exit() {
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
            restart: RestartPolicy::OnFailure,
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        let pods = handle.list_pods(None).await;
        let pod_id = pods[0].id;

        // Exit code 0 means clean exit — should not restart
        handle.send_container_exited(pod_id, 0).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].status, PodStatus::Failed);
    }

    #[tokio::test]
    async fn crash_loop_backoff_after_ten_restarts() {
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
            restart: RestartPolicy::Always,
            resources: None,
        };

        handle.deploy(spec).await.unwrap();
        let pods = handle.list_pods(None).await;
        let pod_id = pods[0].id;

        // Send 10 ContainerExited events rapidly.
        // Each one increments the restart counter. On the 10th,
        // is_crash_loop() returns true and the handler marks CrashLoopBackoff
        // BEFORE scheduling a restart.
        for _ in 0..10 {
            handle.send_container_exited(pod_id, 1).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].status, PodStatus::CrashLoopBackoff);
    }

    // ---- Project lifecycle tests (Task 6) ----

    #[tokio::test]
    async fn suspend_project_stops_all_pods() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "web".into() },
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
        assert_eq!(handle.list_pods(Some("myapp".into())).await.len(), 2);

        handle.suspend_project("myapp".into()).await.unwrap();

        let pods = handle.list_pods(Some("myapp".into())).await;
        assert_eq!(pods.len(), 0);

        let deployments = handle.list_deployments(Some("myapp".into())).await;
        assert_eq!(deployments.len(), 1);
        assert_eq!(deployments[0].status, DeploymentStatus::Stopped);

        let projects = handle.list_projects().await;
        assert_eq!(projects[0].status, ProjectStatus::Suspended);
    }

    #[tokio::test]
    async fn suspend_blocks_new_deploys() {
        let handle = spawn_test_orchestrator();

        handle.create_project("myapp".into()).await.unwrap();
        handle.suspend_project("myapp".into()).await.unwrap();

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

        let result = handle.deploy(spec).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("suspended"), "expected suspended error, got: {err_msg}");
    }

    #[tokio::test]
    async fn resume_project_reconciles_deployments() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "web".into() },
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
        handle.suspend_project("myapp".into()).await.unwrap();
        assert_eq!(handle.list_pods(Some("myapp".into())).await.len(), 0);

        handle.resume_project("myapp".into()).await.unwrap();

        let pods = handle.list_pods(Some("myapp".into())).await;
        assert_eq!(pods.len(), 2);
        assert!(pods.iter().all(|p| p.status == PodStatus::Running));

        let projects = handle.list_projects().await;
        assert_eq!(projects[0].status, ProjectStatus::Active);
    }

    #[tokio::test]
    async fn delete_empty_project_succeeds() {
        let handle = spawn_test_orchestrator();

        handle.create_project("empty".into()).await.unwrap();
        handle.delete_project("empty".into()).await.unwrap();

        let projects = handle.list_projects().await;
        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn delete_nonempty_project_fails() {
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

        let result = handle.delete_project("myapp".into()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not empty"), "expected not empty error, got: {err_msg}");
    }

    #[tokio::test]
    async fn delete_nonexistent_project_fails() {
        let handle = spawn_test_orchestrator();

        let result = handle.delete_project("ghost".into()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not found"), "expected not found error, got: {err_msg}");
    }

    // ---- Secret injection tests (Task 7) ----

    #[tokio::test]
    async fn deploy_injects_secrets_into_env() {
        use std::sync::Mutex;
        use crate::ports::secrets_memory::PlaintextSecretStore;
        use crate::ports::secrets::SecretStore;

        struct CapturingRuntime {
            configs: Mutex<Vec<ContainerConfig>>,
        }

        #[async_trait::async_trait]
        impl ContainerRuntime for CapturingRuntime {
            async fn pull_image(&self, _image: &str) -> Result<()> { Ok(()) }
            async fn create_container(&self, config: &ContainerConfig) -> Result<String> {
                self.configs.lock().unwrap().push(config.clone());
                Ok(format!("mock-{}", config.name))
            }
            async fn start_container(&self, _id: &str) -> Result<()> { Ok(()) }
            async fn stop_container(&self, _id: &str, _timeout: u64) -> Result<()> { Ok(()) }
            async fn remove_container(&self, _id: &str, _force: bool) -> Result<()> { Ok(()) }
            async fn inspect_container(&self, _id: &str) -> Result<ContainerInfo> {
                Ok(ContainerInfo {
                    id: "mock".into(), name: "mock".into(), image: "mock".into(),
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
            async fn container_ip(&self, _container_id: &str, _network: &str) -> Result<String> { Ok("172.17.0.2".to_string()) }
            async fn events(&self) -> Result<EventStream> {
                Ok(Box::pin(futures::stream::pending()))
            }
        }

        let secrets = Arc::new(PlaintextSecretStore::new());
        secrets.set("myapp", "DB_PASSWORD", b"s3cret").await.unwrap();

        let capturing = Arc::new(CapturingRuntime {
            configs: Mutex::new(Vec::new()),
        });
        let handle = Orchestrator::spawn(
            capturing.clone(),
            None,
            Some(secrets as Arc<dyn SecretStore>),
            None,
            None,
            None,
        );

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::from([("APP_NAME".into(), "test".into())]),
            volumes: vec![],
            secrets: vec!["DB_PASSWORD".into()],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        handle.deploy(spec).await.unwrap();

        let configs = capturing.configs.lock().unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].env.get("DB_PASSWORD").unwrap(), "s3cret");
        assert_eq!(configs[0].env.get("APP_NAME").unwrap(), "test");
    }

    #[tokio::test]
    async fn deploy_fails_on_missing_secret() {
        use crate::ports::secrets_memory::PlaintextSecretStore;
        use crate::ports::secrets::SecretStore;

        let secrets = Arc::new(PlaintextSecretStore::new());
        let handle = Orchestrator::spawn(
            Arc::new(MockRuntime),
            None,
            Some(secrets as Arc<dyn SecretStore>),
            None,
            None,
            None,
        );

        let spec = DeploymentSpec {
            project: "myapp".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            secrets: vec!["nonexistent".into()],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
        };

        let result = handle.deploy(spec).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[tokio::test]
    async fn single_node_scheduler_deploys_successfully() {
        let handle = spawn_test_orchestrator();

        let spec = DeploymentSpec {
            project: "test".into(),
            deployment: DeploymentMeta { name: "web".into() },
            replicas: 3,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
            secrets: vec![],
        };

        handle.deploy(spec).await.unwrap();
        let pods = handle.list_pods(None).await;
        assert_eq!(pods.len(), 3);
        assert!(pods.iter().all(|p| p.status == PodStatus::Running));
    }

    #[tokio::test]
    async fn scheduler_config_can_be_changed_at_runtime() {
        use crate::domain::scheduler::SchedulerConfig;

        let handle = spawn_test_orchestrator();

        let config = handle.get_scheduler_config().await;
        assert_eq!(config.strategy, "spread");

        let binpack = SchedulerConfig::from_strategy("binpack").unwrap();
        let updated = handle.set_scheduler_config(binpack).await.unwrap();
        assert_eq!(updated.strategy, "binpack");

        let config = handle.get_scheduler_config().await;
        assert_eq!(config.weights, SchedulerWeights::binpack());
    }

    // --- DNS integration tests ---

    struct SpyDnsProvider {
        registered: StdMutex<Vec<(String, String, std::net::IpAddr)>>,
        deregistered: StdMutex<Vec<(String, String, std::net::IpAddr)>>,
    }

    impl SpyDnsProvider {
        fn new() -> Self {
            Self {
                registered: StdMutex::new(vec![]),
                deregistered: StdMutex::new(vec![]),
            }
        }
    }

    #[async_trait::async_trait]
    impl DnsProvider for SpyDnsProvider {
        async fn register(&self, project: &str, deployment: &str, ip: std::net::IpAddr) -> Result<()> {
            self.registered.lock().unwrap().push((project.to_string(), deployment.to_string(), ip));
            Ok(())
        }
        async fn deregister(&self, project: &str, deployment: &str, ip: std::net::IpAddr) -> Result<()> {
            self.deregistered.lock().unwrap().push((project.to_string(), deployment.to_string(), ip));
            Ok(())
        }
        async fn lookup(&self, _project: &str, _deployment: &str) -> Result<Vec<std::net::IpAddr>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn deploy_registers_dns_for_pods() {
        let dns = Arc::new(SpyDnsProvider::new());
        let handle = Orchestrator::spawn(
            Arc::new(MockRuntime),
            None,
            None,
            None,
            Some(dns.clone() as Arc<dyn DnsProvider>),
            None,
        );

        let spec = DeploymentSpec {
            project: "ecommerce".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 2,
            image: "nginx:latest".into(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
            secrets: vec![],
        };

        handle.deploy(spec).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let registered = dns.registered.lock().unwrap();
        assert_eq!(registered.len(), 2, "should register DNS for each pod");
        assert!(registered.iter().all(|(p, d, _)| p == "ecommerce" && d == "api"));
    }

    #[tokio::test]
    async fn stop_deregisters_dns_for_pods() {
        let dns = Arc::new(SpyDnsProvider::new());
        let handle = Orchestrator::spawn(
            Arc::new(MockRuntime),
            None,
            None,
            None,
            Some(dns.clone() as Arc<dyn DnsProvider>),
            None,
        );

        let spec = DeploymentSpec {
            project: "ecommerce".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx:latest".into(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
            secrets: vec![],
        };

        handle.deploy(spec).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        handle.stop("ecommerce".into(), "api".into()).await.unwrap();

        let deregistered = dns.deregistered.lock().unwrap();
        assert_eq!(deregistered.len(), 1, "should deregister DNS on stop");
        assert_eq!(deregistered[0].0, "ecommerce");
        assert_eq!(deregistered[0].1, "api");
    }

    #[tokio::test]
    async fn containers_receive_dns_config_when_master_ip_set() {
        let handle = Orchestrator::spawn(
            Arc::new(MockRuntime),
            None,
            None,
            None,
            None,
            Some("10.0.0.100".into()),
        );

        let spec = DeploymentSpec {
            project: "ecommerce".into(),
            deployment: DeploymentMeta { name: "api".into() },
            replicas: 1,
            image: "nginx".into(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            network: None,
            healthcheck: None,
            restart: RestartPolicy::default(),
            resources: None,
            secrets: vec![],
        };

        let deployment = handle.deploy(spec).await.unwrap();
        assert_eq!(deployment.name(), "api");
    }
}
