#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ONEAPI_ROOT="${ONEAPI_ROOT:-/home/rrudnick/oneapi_2026.0.0.391}"
SETVARS_SH="${ONEAPI_ROOT}/setvars.sh"

if [[ ! -f "${SETVARS_SH}" ]]; then
    echo "setvars.sh not found at ${SETVARS_SH}" >&2
    exit 1
fi

set +u
source "${SETVARS_SH}" >/dev/null
set -u

if ! command -v sycl-ls >/dev/null 2>&1; then
    echo "sycl-ls is not available after sourcing ${SETVARS_SH}" >&2
    exit 1
fi

echo "Available SYCL devices:"
sycl-ls

cd "${SCRIPT_DIR}"

echo
echo "Running memory example"
cargo run --example memory

echo
echo "Running kernel example"
cargo run --example kernel

echo
echo "Running device_repr example"
cargo run --example device_repr

echo
echo "Running multi_gpu example"
cargo run --example multi_gpu