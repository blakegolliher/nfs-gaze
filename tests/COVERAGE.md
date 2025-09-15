# Test Coverage Report

Generated: 2025-09-15 13:07:35

## Overall Coverage: 62.5%

![Coverage](https://img.shields.io/badge/coverage-62.5%-%25-yellow)

## Summary by File

| File | Coverage | Status |
|------|----------|--------|
| main_unsupported.go | 0.0% | ❌ |
| stats_nolinux.go | 80.6% | ✅ |
| utils_nolinux.go | 70.0% | ⚠️ |

## Detailed Coverage

### main_unsupported.go (0.0%)

<details>
<summary>Function Coverage</summary>

| Function | Coverage |
|----------|----------|
| 🔴 main | 0.0% |

</details>

### stats_nolinux.go (80.6%)

<details>
<summary>Function Coverage</summary>

| Function | Coverage |
|----------|----------|
| 🔴 ParseMountstats | 0.0% |
| 🟠 ParseEvents | 58.1% |
| 🟡 ParseNFSOperation | 69.2% |
| 🟡 parseBytes | 77.8% |
| 🟢 parseOperation | 80.0% |
| 🟢 parseAge | 83.3% |
| 🟢 parseStatsLine | 83.3% |
| 🟢 parseEvents | 83.3% |
| 🟢 parseDeviceLine | 94.1% |
| 🟢 parseLine | 100.0% |
| 🟢 parse | 100.0% |
| 🟢 ParseMountstatsReader | 100.0% |
| 🟢 CalculateDelta | 100.0% |
| 🟢 DisplayStatsSimple | 100.0% |

</details>

### utils_nolinux.go (70.0%)

<details>
<summary>Function Coverage</summary>

| Function | Coverage |
|----------|----------|
| 🔴 MonitoringLoop | 0.0% |
| 🟠 InitFlags | 50.0% |
| 🟢 ParseOperationsFilter | 100.0% |
| 🟢 GetMountsToMonitor | 100.0% |
| 🟢 PrintInitialSummary | 100.0% |

</details>

## Coverage Targets

- 🟢 **Good**: >= 80%
- 🟡 **Acceptable**: >= 60%
- 🟠 **Needs Improvement**: > 0%
- 🔴 **Not Covered**: 0%

## Recommendations

### Functions with No Coverage

- `main` in main_unsupported.go
- `ParseMountstats` in stats_nolinux.go
- `MonitoringLoop` in utils_nolinux.go

### Functions with Low Coverage (<60%)

- `ParseEvents` in stats_nolinux.go (58.1%)
- `InitFlags` in utils_nolinux.go (50.0%)

## Running Tests

```bash
# Run all tests
go test ./...

# Run with coverage (from project root)
go test -coverprofile=tests/coverage.out ./...

# Generate this report using Makefile
make coverage

# Or manually
go run tests/coverage_to_md.go > tests/COVERAGE.md
```
