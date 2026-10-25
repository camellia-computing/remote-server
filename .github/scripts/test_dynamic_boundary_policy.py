from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/dynamic-boundaries.yml"
DOCUMENTATION = ROOT / "docs/dynamic-boundary-testing.md"


workflow = WORKFLOW.read_text(encoding="utf-8")
required = (
    "NIGHTLY_TOOLCHAIN: nightly-2026-08-15",
    "sanitizer: address",
    "sanitizer: thread",
    "-Zbuild-std --locked",
    "--no-run",
    "dynamic_boundaries::malformed_rendezvous_corpus_is_panic_free_and_bounded",
    "database::tests::concurrent_insert_and_read",
    "timeout --signal=TERM --kill-after=30s 10m",
    "halt_on_error=1",
    "fail-fast: false",
)
missing = [marker for marker in required if marker not in workflow]
if missing:
    raise SystemExit(f"Server dynamic boundary workflow is incomplete: {missing}")
for forbidden in ("continue-on-error:", "|| true", "unsafe-allow-abi-mismatch", "pull_request_target:"):
    if forbidden in workflow:
        raise SystemExit(f"Server dynamic boundary workflow contains forbidden marker: {forbidden}")
if workflow.count("timeout-minutes: 30") != 1 or workflow.count("timeout-minutes: 10") != 1:
    raise SystemExit("Server dynamic boundary job timeouts drifted")
documentation = DOCUMENTATION.read_text(encoding="utf-8")
for marker in ("normal Required suite", "nightly-2026-08-15", "ten-minute", "ABI mismatch", "Linux x86_64"):
    if marker not in documentation:
        raise SystemExit(f"Server dynamic boundary documentation is incomplete: {marker}")
print("Server dynamic boundary policy passed")
