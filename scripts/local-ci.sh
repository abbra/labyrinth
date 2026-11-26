#!/usr/bin/env bash
# Run labyrinth's Rust and mdBook checks locally.
# Usage: scripts/local-ci.sh [--no-color] [--no-deps] [all|job ...]

if (( BASH_VERSINFO[0] < 4 )); then
    echo "error: bash 4 or later is required" >&2
    exit 1
fi
set -euo pipefail

RED=$'\033[0;31m'; GREEN=$'\033[0;32m'; YELLOW=$'\033[1;33m'
BLUE=$'\033[0;34m'; BOLD=$'\033[1m'; NC=$'\033[0m'
NO_DEPS=0
REQUESTED=()
while (($#)); do
    case "$1" in
        --no-color) RED=''; GREEN=''; YELLOW=''; BLUE=''; BOLD=''; NC='' ;;
        --no-deps) NO_DEPS=1 ;;
        *) REQUESTED+=("$1") ;;
    esac
    shift
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    export CARGO_TARGET_DIR
    echo "Using CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
fi
if [[ "${NO_COLOR:-}" == 1 ]]; then
    RED=''; GREEN=''; YELLOW=''; BLUE=''; BOLD=''; NC=''
    export CARGO_TERM_COLOR=never
else
    export CARGO_TERM_COLOR=always
fi
export RUSTDOCFLAGS="-D warnings"

ALL_JOBS=(build fmt clippy test doc examples wasm wasm-dist)
declare -A JOB_STATUS=() JOB_SECS=()
FAILED_JOBS=()
declare -A JOB_DEPS=( [clippy]=build [test]=build [doc]=test [examples]=build [wasm]=build [wasm-dist]=wasm )
step() { printf '\n%s▶ %s%s\n' "$BOLD$BLUE" "$*" "$NC"; }
ok() { printf '%s✔ %s%s\n' "$GREEN" "$*" "$NC"; }
warn() { printf '%s! %s%s\n' "$YELLOW" "$*" "$NC"; }
fail() { printf '%s✘ %s%s\n' "$RED" "$*" "$NC" >&2; }

run_job() {
    local name=$1; shift
    if (( ! NO_DEPS )); then
        local dep
        for dep in ${JOB_DEPS[$name]:-}; do
            [[ -n "${JOB_STATUS[$dep]:-}" ]] || dispatch_job "$dep"
            if [[ "${JOB_STATUS[$dep]}" != PASS ]]; then
                JOB_STATUS[$name]=SKIP
                warn "[$name] skipped: dependency '$dep' did not pass"
                return 0
            fi
        done
    fi
    local start=$SECONDS
    step "[$name]"
    if "$@"; then
        JOB_STATUS[$name]=PASS
        JOB_SECS[$name]=$((SECONDS - start))
        ok "[$name] passed (${JOB_SECS[$name]}s)"
    else
        JOB_STATUS[$name]=FAIL
        JOB_SECS[$name]=$((SECONDS - start))
        FAILED_JOBS+=("$name")
        fail "[$name] failed (${JOB_SECS[$name]}s)"
    fi
}

job_build() { cargo build --workspace --release; }
job_fmt() { cargo fmt --all -- --check; }
job_clippy() { cargo clippy --workspace --all-targets -- -D warnings; }
job_test() { cargo test -q --release --workspace; }
job_doc() {
    if ! command -v mdbook >/dev/null 2>&1; then
        fail "mdbook not found (install with: cargo install mdbook)"
        return 1
    fi
    cargo build -q --release -p labyrinth &&
        mdbook build docs && mdbook test docs -L target/release
}
job_examples() {
    local maze='S.#
#.#
#.G
'
    printf '%s' "$maze" | cargo run -q --release --example solve >/dev/null &&
        printf '%s' "$maze" | cargo run -q --release --example bench >/dev/null
}
job_wasm() {
    local target_dir="${CARGO_TARGET_DIR:-target}"
    cargo build --release -p labyrinth-wasm --target wasm32-unknown-unknown &&
        wasm-bindgen --target web --out-dir wasm/pkg "$target_dir/wasm32-unknown-unknown/release/labyrinth_wasm.wasm"
}
job_wasm_dist() {
    local dist_dir=target/wasm-dist
    rm -rf "$dist_dir"
    mkdir -p "$dist_dir/server/src" "$dist_dir/wasm/pkg" "$dist_dir/wasm-demo"
    cp server/src/index.html "$dist_dir/server/src/"
    cp wasm-demo/index.html "$dist_dir/wasm-demo/"
    cp -a wasm/pkg/. "$dist_dir/wasm/pkg/"
    test -s "$dist_dir/server/src/index.html" &&
        test -s "$dist_dir/wasm-demo/index.html" &&
        test -s "$dist_dir/wasm/pkg/labyrinth_wasm.js" &&
        test -s "$dist_dir/wasm/pkg/labyrinth_wasm_bg.wasm" &&
        printf 'Copyable browser-only distribution: %s\n' "$dist_dir"
}

dispatch_job() {
    case "$1" in
        build) run_job build job_build ;;
        fmt) run_job fmt job_fmt ;;
        clippy) run_job clippy job_clippy ;;
        test) run_job test job_test ;;
        doc) run_job doc job_doc ;;
        examples) run_job examples job_examples ;;
        wasm) run_job wasm job_wasm ;;
        wasm-dist) run_job wasm-dist job_wasm_dist ;;
    esac
}

if ((${#REQUESTED[@]} == 0)); then
    printf 'Usage: %s [--no-color] [--no-deps] <all|job ...>\nJobs: %s\n' "$0" "${ALL_JOBS[*]}"
    exit 0
fi
if [[ "${REQUESTED[0]}" == --list ]]; then printf '%s\n' all "${ALL_JOBS[@]}"; exit 0; fi
if [[ "${REQUESTED[0]}" == --help || "${REQUESTED[0]}" == -h ]]; then
    printf 'Usage: %s [--no-color] [--no-deps] <all|job ...>\n\nJobs: %s\n' "$0" "${ALL_JOBS[*]}"
    exit 0
fi
JOBS=()
for job in "${REQUESTED[@]}"; do
    if [[ "$job" == all ]]; then JOBS+=("${ALL_JOBS[@]}"); else JOBS+=("$job"); fi
done
start=$SECONDS
for job in "${JOBS[@]}"; do dispatch_job "$job"; done
printf '\n%sCI Summary%s\n' "$BOLD" "$NC"
for job in "${ALL_JOBS[@]}"; do
    [[ -n "${JOB_STATUS[$job]:-}" ]] || continue
    printf '  %-12s %s (%ss)\n' "$job" "${JOB_STATUS[$job]}" "${JOB_SECS[$job]:-0}"
done
printf 'Total time: %ss\n' "$((SECONDS - start))"
if ((${#FAILED_JOBS[@]})); then
    fail "FAILED: ${FAILED_JOBS[*]}"
    exit 1
fi
ok 'All requested jobs passed.'
