use async_trait::async_trait;

use crate::domain::models::{Certificate, Route, SubnetAllocation};
use crate::error::Result;

#[async_trait]
pub trait RouteStore: Send + Sync {
    async fn insert_route(&self, route: &Route) -> Result<()>;
    async fn get_route(&self, domain: &str) -> Result<Option<Route>>;
    async fn list_routes(&self, project: Option<&str>) -> Result<Vec<Route>>;
    async fn delete_route(&self, domain: &str) -> Result<bool>;

    async fn upsert_certificate(&self, cert: &Certificate) -> Result<()>;
    async fn get_certificate(&self, domain: &str) -> Result<Option<Certificate>>;
    async fn list_expiring_certificates(&self, within_days: i64) -> Result<Vec<Certificate>>;
    async fn delete_certificate(&self, domain: &str) -> Result<bool>;

    async fn allocate_subnet(&self, alloc: &SubnetAllocation) -> Result<()>;
    async fn get_node_subnet(
        &self,
        node_id: &str,
        project: &str,
    ) -> Result<Option<SubnetAllocation>>;
    async fn list_subnets(&self) -> Result<Vec<SubnetAllocation>>;
    async fn deallocate_subnet(&self, node_id: &str, project: &str) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn RouteStore) {}
    }
}
