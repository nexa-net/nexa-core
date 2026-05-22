use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::domain::models::{Certificate, Route, SubnetAllocation};
use crate::error::{NexaError, Result};
use super::route_store::RouteStore;

pub struct InMemoryRouteStore {
    routes: RwLock<HashMap<String, Route>>,
    certificates: RwLock<HashMap<String, Certificate>>,
    subnets: RwLock<Vec<SubnetAllocation>>,
}

impl InMemoryRouteStore {
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(HashMap::new()),
            certificates: RwLock::new(HashMap::new()),
            subnets: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl RouteStore for InMemoryRouteStore {
    async fn insert_route(&self, route: &Route) -> Result<()> {
        let mut routes = self.routes.write().unwrap();
        if routes.contains_key(&route.domain) {
            return Err(NexaError::RouteAlreadyExists(route.domain.clone()));
        }
        routes.insert(route.domain.clone(), route.clone());
        Ok(())
    }

    async fn get_route(&self, domain: &str) -> Result<Option<Route>> {
        let routes = self.routes.read().unwrap();
        Ok(routes.get(domain).cloned())
    }

    async fn list_routes(&self, project: Option<&str>) -> Result<Vec<Route>> {
        let routes = self.routes.read().unwrap();
        Ok(routes.values()
            .filter(|r| project.map_or(true, |p| r.project == p))
            .cloned()
            .collect())
    }

    async fn delete_route(&self, domain: &str) -> Result<bool> {
        let mut routes = self.routes.write().unwrap();
        Ok(routes.remove(domain).is_some())
    }

    async fn upsert_certificate(&self, cert: &Certificate) -> Result<()> {
        let mut certs = self.certificates.write().unwrap();
        certs.insert(cert.domain.clone(), cert.clone());
        Ok(())
    }

    async fn get_certificate(&self, domain: &str) -> Result<Option<Certificate>> {
        let certs = self.certificates.read().unwrap();
        Ok(certs.get(domain).cloned())
    }

    async fn list_expiring_certificates(&self, within_days: i64) -> Result<Vec<Certificate>> {
        let certs = self.certificates.read().unwrap();
        let threshold = Utc::now() + Duration::days(within_days);
        Ok(certs.values().filter(|c| c.expires_at <= threshold).cloned().collect())
    }

    async fn delete_certificate(&self, domain: &str) -> Result<bool> {
        let mut certs = self.certificates.write().unwrap();
        Ok(certs.remove(domain).is_some())
    }

    async fn allocate_subnet(&self, alloc: &SubnetAllocation) -> Result<()> {
        let mut subnets = self.subnets.write().unwrap();
        if subnets.iter().any(|s| s.node_id == alloc.node_id && s.project == alloc.project) {
            return Err(NexaError::Network(format!("subnet already allocated for node {} project {}", alloc.node_id, alloc.project)));
        }
        if subnets.iter().any(|s| s.subnet == alloc.subnet) {
            return Err(NexaError::Network(format!("subnet {} already in use", alloc.subnet)));
        }
        subnets.push(alloc.clone());
        Ok(())
    }

    async fn get_node_subnet(&self, node_id: &str, project: &str) -> Result<Option<SubnetAllocation>> {
        let subnets = self.subnets.read().unwrap();
        Ok(subnets.iter().find(|s| s.node_id == node_id && s.project == project).cloned())
    }

    async fn list_subnets(&self) -> Result<Vec<SubnetAllocation>> {
        Ok(self.subnets.read().unwrap().clone())
    }

    async fn deallocate_subnet(&self, node_id: &str, project: &str) -> Result<bool> {
        let mut subnets = self.subnets.write().unwrap();
        let len = subnets.len();
        subnets.retain(|s| !(s.node_id == node_id && s.project == project));
        Ok(subnets.len() < len)
    }
}
