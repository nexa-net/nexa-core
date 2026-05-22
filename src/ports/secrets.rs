use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn set(&self, project: &str, name: &str, value: &[u8]) -> Result<()>;
    async fn get(&self, project: &str, name: &str) -> Result<Option<Vec<u8>>>;
    async fn list(&self, project: &str) -> Result<Vec<String>>;
    async fn delete(&self, project: &str, name: &str) -> Result<()>;
}
