<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/white_logo.png" width="200">
  <source media="(prefers-color-scheme: light)" srcset="assets/black_logo.png" width="200">
  <img alt="NexaNet" src="assets/black_logo.png" width="200">
</picture>

# nexa-core

**Core domain types, traits, and orchestrator for NexaNet**

[![CI](https://github.com/nexa-net/nexa-core/actions/workflows/ci.yml/badge.svg)](https://github.com/nexa-net/nexa-core/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

nexa-core defines the shared domain model, port traits, and actor-based orchestrator
that power the NexaNet container platform. It is a library crate with no binary --
consumed by [nexad](https://github.com/nexa-net/nexad) (the daemon) and
[nexa-cli](https://github.com/nexa-net/nexa-cli) (the CLI).

</div>

---

## Features

- **Domain models** -- Project, Deployment, Pod, Node, Route, Certificate, SubnetAllocation with full serde support
- **Actor-model orchestrator** -- async command loop over `mpsc`/`oneshot` channels with 30+ command variants (deploy, scale, stop, secrets, routes, health, scheduling)
- **Port traits (hexagonal architecture)** -- `ContainerRuntime` (14 methods), `StateStore`, `SecretStore`, `ClusterTransport`, `DnsProvider`, `RouteStore`, `ProxyBackend`
- **YAML deployment specs** -- declarative config with DNS-name validation, resource limits, health probes, restart policies, volumes, secrets, and network config
- **Weighted scheduler** -- spread and bin-pack strategies with configurable CPU/memory/load/failure weights
- **Health tracking** -- probe-based health state machine with configurable intervals, timeouts, and retry thresholds
- **Restart policies** -- `always`, `on_failure`, and `never` with exponential backoff and crash-loop detection
- **177 unit tests**

## Architecture

```
nexa-core/
  src/
    config.rs            -- YAML spec parser and validator
    duration.rs          -- Human-friendly duration parsing (e.g. "10s", "5m")
    error.rs             -- NexaError type and Result alias
    domain/
      models/
        project.rs       -- Project with Active/Suspended status
        deployment.rs    -- DeploymentSpec, volumes, health checks, resources
        pod.rs           -- Pod lifecycle: Pending -> Running -> Stopped/Failed
        node.rs          -- Node with role (Master/Worker), resources, heartbeat
        route.rs         -- Route, Certificate, TlsMode, SubnetAllocation
      orchestrator.rs    -- Actor loop: Command enum + OrchestratorHandle
      scheduler.rs       -- WeightedScheduler with spread/binpack strategies
      health.rs          -- HealthTracker state machine
      restart.rs         -- RestartState with exponential backoff
    ports/
      runtime.rs         -- ContainerRuntime trait (14 async methods)
      state.rs           -- StateStore trait (projects, deployments, pods, nodes)
      secrets.rs         -- SecretStore trait (set/get/list/delete)
      cluster.rs         -- ClusterTransport trait (register, heartbeat, assign)
      dns.rs             -- DnsProvider trait (register/deregister/lookup)
      proxy.rs           -- ProxyBackend trait (apply_routes, reload, health)
      route_store.rs     -- RouteStore trait
      *_memory.rs        -- In-memory test implementations
```

The crate follows **hexagonal architecture** (ports and adapters). All external
concerns are expressed as traits in `src/ports/`. The orchestrator and domain logic
depend only on these traits, never on concrete implementations. Adapters live in
[nexad](https://github.com/nexa-net/nexad).

## Usage

### As a dependency

```toml
[dependencies]
nexa-core = { git = "https://github.com/nexa-net/nexa-core" }
```

### Deployment spec (YAML)

```yaml
project: ecommerce

deployment:
  name: api

replicas: 3
image: ghcr.io/company/api:latest

ports:
  - 3000

env:
  DATABASE_URL: "postgres://localhost/ecommerce"
  REDIS_URL: "redis://localhost:6379"

network:
  public: true
  domain: api.example.com
  https: true

healthcheck:
  path: /health
  interval: 10s

restart: always
```

### Parsing a spec

```rust
use std::path::Path;
use nexa_core::config::parse_deployment_file;

let spec = parse_deployment_file(Path::new("deploy.yaml"))?;
println!("Deploying {} to project {}", spec.deployment.name, spec.project);
```

### Spawning the orchestrator

```rust
use nexa_core::domain::orchestrator::Orchestrator;

let handle = Orchestrator::spawn(
    runtime,        // Arc<dyn ContainerRuntime>
    Some(store),    // Arc<dyn StateStore>
    Some(secrets),  // Arc<dyn SecretStore>
    Some(transport),// Arc<dyn ClusterTransport>
    dns,            // Option<Arc<dyn DnsProvider>>
    master_ip,      // Option<String>
    proxy,          // Option<Arc<dyn ProxyBackend>>
    route_store,    // Option<Arc<dyn RouteStore>>
);

// All interaction goes through the handle (thread-safe, cloneable)
let deployment = handle.deploy(spec).await?;
let pods = handle.list_pods(None).await;
```

### Key port traits

```rust
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    fn runtime_name(&self) -> &'static str;
    async fn pull_image(&self, image: &str) -> Result<()>;
    async fn create_container(&self, config: &ContainerConfig) -> Result<String>;
    async fn start_container(&self, id: &str) -> Result<()>;
    async fn stop_container(&self, id: &str, timeout_secs: u64) -> Result<()>;
    async fn remove_container(&self, id: &str, force: bool) -> Result<()>;
    async fn inspect_container(&self, id: &str) -> Result<ContainerInfo>;
    async fn logs(&self, id: &str, tail: Option<u64>) -> Result<LogStream>;
    async fn container_exists(&self, name: &str) -> Result<bool>;
    async fn create_network(&self, name: &str) -> Result<String>;
    async fn remove_network(&self, name: &str) -> Result<()>;
    async fn connect_to_network(&self, container_id: &str, network: &str) -> Result<()>;
    async fn container_ip(&self, container_id: &str, network: &str) -> Result<String>;
    async fn events(&self) -> Result<EventStream>;
}

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn insert_project(&self, project: &Project) -> Result<()>;
    async fn list_projects(&self) -> Result<Vec<Project>>;
    async fn insert_deployment(&self, deployment: &Deployment) -> Result<()>;
    async fn list_deployments(&self, project: Option<&str>) -> Result<Vec<Deployment>>;
    // ... 20+ methods for projects, deployments, pods, nodes, cluster config
}
```

## Development

```bash
# Build
cargo build

# Run all 177 tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## Related Repositories

| Repository | Description |
|---|---|
| [nexad](https://github.com/nexa-net/nexad) | Daemon -- concrete adapters, REST API, clustering |
| [nexa-cli](https://github.com/nexa-net/nexa-cli) | CLI tool for deploying and managing containers |
| [nexa-proxy](https://github.com/nexa-net/nexa-proxy) | Lightweight reverse proxy with weighted load balancing |

## License

Apache-2.0 -- see [LICENSE](LICENSE) for details.
