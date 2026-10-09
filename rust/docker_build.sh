#!/bin/bash

docker run \
  --rm \
  --shm-size=2g \
  --user "$(id -u)":"$(id -g)" \
  -e CARGO_TARGET_DIR=/builddir/docker-target \
  -e C_BUILD_DIR=/builddir/.build/docker \
  -v "$PWD":/builddir \
  -w /builddir \
  rust:1.98.1 \
  bash build.sh
