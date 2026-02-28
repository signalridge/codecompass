#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: benchmarks/retrieval/run_retrieval_gate.sh [--workspace <path>] [--output <path>] [--dry-run]

Runs retrieval evaluation gate on a workspace using benchmark defaults:
- suite:    benchmarks/retrieval/query-pack.v1.json
- baseline: benchmarks/retrieval/baseline.v1.json
- policy:   benchmarks/retrieval/gate-policy.v1.json
USAGE
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"
WORKSPACE="${REPO_ROOT}/testdata/fixtures/rust-sample"
OUTPUT="${REPO_ROOT}/target/retrieval-eval-report.json"
DRY_RUN="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --workspace)
      WORKSPACE="$2"
      shift 2
      ;;
    --output)
      OUTPUT="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

mkdir -p "$(dirname -- "$OUTPUT")"

set -x
cargo run -p cruxe -- init --path "$WORKSPACE"
cargo run -p cruxe -- index --path "$WORKSPACE" --force

cmd=(
  cargo run -p cruxe -- eval retrieval
  --workspace "$WORKSPACE"
  --suite "${SCRIPT_DIR}/query-pack.v1.json"
  --baseline "${SCRIPT_DIR}/baseline.v1.json"
  --policy "${SCRIPT_DIR}/gate-policy.v1.json"
  --ref live
  --limit 10
  --output "$OUTPUT"
)

if [[ "$DRY_RUN" == "true" ]]; then
  cmd+=(--dry-run)
fi

"${cmd[@]}"
set +x

echo "retrieval-eval report: $OUTPUT"
