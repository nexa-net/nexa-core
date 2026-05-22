use criterion::{Criterion, criterion_group, criterion_main};
use nexa_core::config::parse_deployment;

const MINIMAL_SPEC: &str = r#"
project: myapp
deployment:
  name: api
image: nginx:latest
"#;

const FULL_SPEC: &str = r#"
project: ecommerce
deployment:
  name: api
replicas: 3
image: ghcr.io/company/api:latest
ports:
  - 3000
env:
  NODE_ENV: production
  DATABASE_URL: "postgres://localhost/db"
  REDIS_URL: "redis://localhost"
secrets:
  - DATABASE_URL
  - STRIPE_KEY
volumes:
  - name: data
    mount: /app/data
  - path: /host/uploads
    mount: /app/uploads
    readonly: true
network:
  public: true
  domain: api.example.com
  https: true
healthcheck:
  path: /health
  interval: 10s
  timeout: 5s
  retries: 3
restart: always
resources:
  memory: 512m
  cpu: 0.5
"#;

fn bench_parse_minimal_spec(c: &mut Criterion) {
    c.bench_function("parse_minimal_spec", |b| {
        b.iter(|| parse_deployment(MINIMAL_SPEC).unwrap());
    });
}

fn bench_parse_full_spec(c: &mut Criterion) {
    c.bench_function("parse_full_spec", |b| {
        b.iter(|| parse_deployment(FULL_SPEC).unwrap());
    });
}

criterion_group!(benches, bench_parse_minimal_spec, bench_parse_full_spec);
criterion_main!(benches);
