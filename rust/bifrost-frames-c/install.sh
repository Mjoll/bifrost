#!/bin/bash

set -e

cp ../target/release/libbifrost_frames_c.so /usr/lib64/.
cp pkgconfig/bifrost-frames.pc /usr/lib64/pkgconfig/.
cp include/bifrost_frames.h /usr/include/bifrost_frames.h
