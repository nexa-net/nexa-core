use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use super::secrets::SecretStore;
use crate::error::Result;

pub struct PlaintextSecretStore {
    data: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl PlaintextSecretStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SecretStore for PlaintextSecretStore {
    async fn set(&self, project: &str, name: &str, value: &[u8]) -> Result<()> {
        let mut map = self.data.lock().unwrap();
        map.insert((project.to_string(), name.to_string()), value.to_vec());
        Ok(())
    }

    async fn get(&self, project: &str, name: &str) -> Result<Option<Vec<u8>>> {
        let map = self.data.lock().unwrap();
        Ok(map.get(&(project.to_string(), name.to_string())).cloned())
    }

    async fn list(&self, project: &str) -> Result<Vec<String>> {
        let map = self.data.lock().unwrap();
        let names: Vec<String> = map
            .keys()
            .filter(|(p, _)| p == project)
            .map(|(_, n)| n.clone())
            .collect();
        Ok(names)
    }

    async fn delete(&self, project: &str, name: &str) -> Result<()> {
        let mut map = self.data.lock().unwrap();
        map.remove(&(project.to_string(), name.to_string()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip_set_and_get() {
        let store = PlaintextSecretStore::new();
        store.set("myapp", "DB_PASSWORD", b"s3cret").await.unwrap();

        let value = store.get("myapp", "DB_PASSWORD").await.unwrap();
        assert_eq!(value, Some(b"s3cret".to_vec()));

        let missing = store.get("myapp", "NONEXISTENT").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn project_isolation() {
        let store = PlaintextSecretStore::new();
        store.set("app1", "SECRET", b"value1").await.unwrap();
        store.set("app2", "SECRET", b"value2").await.unwrap();

        let v1 = store.get("app1", "SECRET").await.unwrap().unwrap();
        let v2 = store.get("app2", "SECRET").await.unwrap().unwrap();
        assert_eq!(v1, b"value1");
        assert_eq!(v2, b"value2");

        // Listing one project doesn't leak the other
        let names1 = store.list("app1").await.unwrap();
        assert_eq!(names1, vec!["SECRET".to_string()]);
    }

    #[tokio::test]
    async fn list_and_delete() {
        let store = PlaintextSecretStore::new();
        store.set("proj", "A", b"1").await.unwrap();
        store.set("proj", "B", b"2").await.unwrap();

        let mut names = store.list("proj").await.unwrap();
        names.sort();
        assert_eq!(names, vec!["A".to_string(), "B".to_string()]);

        store.delete("proj", "A").await.unwrap();

        let names = store.list("proj").await.unwrap();
        assert_eq!(names, vec!["B".to_string()]);

        let deleted = store.get("proj", "A").await.unwrap();
        assert!(deleted.is_none());
    }
}
