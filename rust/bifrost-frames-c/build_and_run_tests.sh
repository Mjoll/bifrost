#!/bin/bash

set -e

if [ "$1" == "debug" ]; then
  CONFIG="debug"
  FLAGS="-g -O0"
elif [ "$1" == "release" ]; then
  CONFIG="release"
  FLAGS="-g -O2"
else
  echo "Usage: $0 <debug|release>"
  exit 1
fi

if [ -z "$CC" ]; then
  if which gcc &> /dev/null; then
    CC=gcc
  elif which clang &> /dev/null; then
    CC=clang
  else
    echo "ERROR: No C compiler found in path. Tried (gcc and clang)"
    exit 1
  fi
  echo "Using auto-detected C compiler $CC"
elif ! which "${CC}" &> /dev/null; then
  echo "ERROR: CC variable set to \"${CC}\" but \"${CC}\" is not in PATH"
  exit 1
fi

# Convert to absolute path before changing directory
if [ -n "${CARGO_TARGET_DIR}" ]; then
  CARGO_TARGET_DIR=$(cd "${CARGO_TARGET_DIR}"; pwd)
fi


# Convert to absolute path before changing directory
if [ -n "${C_BUILD_DIR}" ]; then
  # Make sure directory exists before resolving absolute path
  [ -d "${C_BUILD_DIR}" ] || mkdir -p "${C_BUILD_DIR}"
  C_BUILD_DIR=$(cd "${C_BUILD_DIR}"; pwd)
fi

cd $(dirname $0)

# If not set use default path relative to script dir
if [ -z "${CARGO_TARGET_DIR}" ]; then
  DEFAULT_CARGO_TARGET_DIR="../target"
  CARGO_TARGET_DIR=$(cd ${DEFAULT_CARGO_TARGET_DIR}; pwd)
fi

# If not set use default path relative to script dir
if [ -z "${C_BUILD_DIR}" ]; then
  # Make sure directory exists before resolving absolute path
  DEFAULT_C_BUILD_DIR="../.build/local"
  [ -d "${DEFAULT_C_BUILD_DIR}" ] || mkdir -p "${DEFAULT_C_BUILD_DIR}"
  C_BUILD_DIR=$(cd ${DEFAULT_C_BUILD_DIR}; pwd)
fi

RUST_TARGET_DIR="${CARGO_TARGET_DIR}/${CONFIG}"
BUILD_OUTPUT_DIR="$(pwd)/../.build"

LDFLAGS="-L${RUST_TARGET_DIR} -lbifrost_frames_c"
CFLAGS="$FLAGS -Wall -Werror -Iinclude"

for PROGRAM in test_producer test_consumer; do
  $CC $CFLAGS tests/interrupt_handler.c tests/${PROGRAM}.c $LDFLAGS -o "${BUILD_OUTPUT_DIR}/${PROGRAM}"
done

export LD_LIBRARY_PATH="${RUST_TARGET_DIR}"

set +e

cd "${BUILD_OUTPUT_DIR}"
./test_producer &

PRODUCER_PID=$!

echo "Ensure the queue is created"
sleep 1

./test_consumer

CONSUMER_EXIT_CODE=$?

wait ${PRODUCER_PID}
PRODUCER_EXIT_CODE=$?

# Check producer first since failures there will likely cause consumer to fail
if [ "$PRODUCER_EXIT_CODE" -ne 0 ]; then
  echo "Test producer failed!"
  exit 1
fi

if [ "$CONSUMER_EXIT_CODE" -ne 0 ]; then
  echo "Test consumer failed!"
  exit 1
fi

echo "Test OK"


