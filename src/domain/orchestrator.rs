use tokio::sync::{mpsc, oneshot};

use crate::domain::models::*;
use crate::error::{NexaError, Result};
use crate::ports::runtime::LogStream;

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
}
