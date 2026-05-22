use std::net::IpAddr;

use async_trait::async_trait;

use crate::error::Result;

#[async_trait]
pub trait DnsProvider: Send + Sync {
    async fn register(&self, project: &str, deployment: &str, ip: IpAddr) -> Result<()>;
    async fn deregister(&self, project: &str, deployment: &str, ip: IpAddr) -> Result<()>;
    async fn lookup(&self, project: &str, deployment: &str) -> Result<Vec<IpAddr>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn DnsProvider) {}
    }

    #[test]
    fn ipaddr_supports_v4_and_v6() {
        let v4: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let v6: IpAddr = "::1".parse().unwrap();
        assert!(v4.is_ipv4());
        assert!(v6.is_ipv6());
    }
}
