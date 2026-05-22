window.BENCHMARK_DATA = {
  "lastUpdate": 1779487624092,
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
      }
    ]
  }
}