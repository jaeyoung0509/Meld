# Performance Regression Gates

This repository provides deterministic REST + gRPC performance smoke gates to detect major regressions early.

## Scope

- REST scenario: `k6` against `/health`
- gRPC scenario: `ghz` against `meld.v1.Greeter/SayHello`
- Outputs saved under `target/perf`

## Local Run (One Command)

Prerequisites:

- `k6`
- `ghz`
- `python3`

Run:

```bash
./scripts/perf_gate.sh
```

Artifacts:

- `target/perf/rest-k6-summary.json`
- `target/perf/grpc-ghz-summary.json`
- `target/perf/grpc-evaluation.txt`
- `target/perf/summary.txt`

## CI Run

Dedicated workflow:

- `.github/workflows/perf.yml`

Trigger mode:

- `workflow_dispatch` (manual)

This keeps perf checks deterministic and isolates noisy throughput tests from standard PR CI.

## Default Thresholds

- REST p95 latency: `<= 120ms`
- REST error rate: `<= 0.01`
- gRPC p95 latency: `<= 120ms`
- gRPC error rate: `<= 0.01`

## Tuning Guidance

If false positives occur on slower hardware or noisy runners:

1. increase load duration first (`MELD_PERF_REST_DURATION`) to reduce variance
2. reduce concurrency/requests (`MELD_PERF_REST_VUS`, `MELD_PERF_GRPC_CONCURRENCY`, `MELD_PERF_GRPC_REQUESTS`)
3. relax thresholds incrementally (5-10ms p95 steps or 0.005 error-rate steps)
4. record new baseline values in this document before merging changes

Keep threshold updates explicit and justified in PR descriptions.

## Notes And Limits

- These are smoke/regression gates, not full-scale capacity benchmarks.
- They do not model cross-region traffic or production replay.
- They are intended to catch clear regressions in latency and error behavior.
