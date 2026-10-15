#!/bin/bash

set -e

cargo build --release
cargo test --release
bifrost-frames-c/build_and_run_tests.sh release
