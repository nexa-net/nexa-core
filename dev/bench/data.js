window.BENCHMARK_DATA = {
  "lastUpdate": 1779655177921,
  "repoUrl": "https://github.com/nexa-net/nexa-core",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "nassime.abdiou@icloud.com",
            "name": "Nassime Abdiou",
            "username": "na2sime"
          },
          "committer": {
            "email": "nassime.abdiou@icloud.com",
            "name": "Nassime Abdiou",
            "username": "na2sime"
          },
          "distinct": true,
          "id": "3ee1cb9c36c53accdf4cadcf5a36614df93fd63e",
          "message": "style: fix formatting in scheduler benchmark",
          "timestamp": "2026-05-22T22:04:28+02:00",
          "tree_id": "3247fe2f668bcf85a466976f7f93813c55098839",
          "url": "https://github.com/nexa-net/nexa-core/commit/3ee1cb9c36c53accdf4cadcf5a36614df93fd63e"
        },
        "date": 1779487255270,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_minimal_spec",
            "value": 30496,
            "range": "± 1216",
            "unit": "ns/iter"
          },
          {
            "name": "parse_full_spec",
            "value": 70944,
            "range": "± 547",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_spread/5",
            "value": 311,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_spread/20",
            "value": 1220,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_binpack/5",
            "value": 313,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_binpack/20",
            "value": 1229,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "nassime.abdiou@icloud.com",
            "name": "Nassime Abdiou",
            "username": "na2sime"
          },
          "committer": {
            "email": "nassime.abdiou@icloud.com",
            "name": "Nassime Abdiou",
            "username": "na2sime"
          },
          "distinct": true,
          "id": "952fc6fe28a11ffea35cee86100c20cf55a43168",
          "message": "feat: add command field to ContainerConfig",
          "timestamp": "2026-05-23T00:05:27+02:00",
          "tree_id": "a1fce8f09ebb7b73fa798f015e7d008e60ff2474",
          "url": "https://github.com/nexa-net/nexa-core/commit/952fc6fe28a11ffea35cee86100c20cf55a43168"
        },
        "date": 1779487623309,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_minimal_spec",
            "value": 28046,
            "range": "± 211",
            "unit": "ns/iter"
          },
          {
            "name": "parse_full_spec",
            "value": 66655,
            "range": "± 718",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_spread/5",
            "value": 241,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_spread/20",
            "value": 953,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_binpack/5",
            "value": 240,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_binpack/20",
            "value": 951,
            "range": "± 11",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "nassime.abdiou@icloud.com",
            "name": "Nassime Abdiou",
            "username": "na2sime"
          },
          "committer": {
            "email": "nassime.abdiou@icloud.com",
            "name": "Nassime Abdiou",
            "username": "na2sime"
          },
          "distinct": true,
          "id": "0a9c4e2ad06173bacbd180653c969f5cc65aa4d0",
          "message": "feat: add MetricsPort to Orchestrator with handler instrumentation\n\nAdds an optional MetricsPort field (9th parameter) to Orchestrator::spawn,\ninstruments handle_deploy, handle_scale, handle_stop, handle_remove_deployment,\nhandle_container_exited, and select_node with deployment op, container event,\nand schedule decision metrics; also adds update_gauge_counts helper.",
          "timestamp": "2026-05-24T10:57:03+02:00",
          "tree_id": "2d0eeffb56975e18da5105149b7076f6bacc4a01",
          "url": "https://github.com/nexa-net/nexa-core/commit/0a9c4e2ad06173bacbd180653c969f5cc65aa4d0"
        },
        "date": 1779614282773,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_minimal_spec",
            "value": 29150,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "parse_full_spec",
            "value": 66284,
            "range": "± 6204",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_spread/5",
            "value": 342,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_spread/20",
            "value": 1337,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_binpack/5",
            "value": 340,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_binpack/20",
            "value": 1340,
            "range": "± 4",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "nassime.abdiou@icloud.com",
            "name": "Nassime Abdiou",
            "username": "na2sime"
          },
          "committer": {
            "email": "nassime.abdiou@icloud.com",
            "name": "Nassime Abdiou",
            "username": "na2sime"
          },
          "distinct": true,
          "id": "01a5a3f544fa2fe09b58136587c381b85802dc98",
          "message": "style: apply cargo fmt formatting to metrics port and orchestrator",
          "timestamp": "2026-05-24T19:09:07+02:00",
          "tree_id": "c044e94f047b8319dc789b940d7340e350303c0e",
          "url": "https://github.com/nexa-net/nexa-core/commit/01a5a3f544fa2fe09b58136587c381b85802dc98"
        },
        "date": 1779655177058,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_minimal_spec",
            "value": 29479,
            "range": "± 303",
            "unit": "ns/iter"
          },
          {
            "name": "parse_full_spec",
            "value": 70518,
            "range": "± 2290",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_spread/5",
            "value": 318,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_spread/20",
            "value": 1230,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_binpack/5",
            "value": 320,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "scheduler_binpack/20",
            "value": 1234,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}