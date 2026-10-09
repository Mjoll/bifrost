# Bifrost

Bifrost is a set of Rust crates for inter-process communication over shared
memory. It consists of two layers:

- **`bifrost`** – general purpose shared memory primitives: named segments,
  structured memory layouts, cross-process mutexes and events, and queues. It
  has no knowledge of video or audio and can be used to build any kind of
  shared memory IPC.
- **`bifrost-frames`** – a zero-copy video and audio frame transport built on
  top of `bifrost`. A producer allocates frames directly in shared memory,
  fills them in, and pushes them onto a named stream; consumers in other
  processes pop the same frames, read them in place, and hand them back to a
  pool for reuse. It also ships a C API, so it can be used from C, C++ or
  anything else with a C FFI.

## Frame transport features

- **Zero-copy frames** – video and audio buffers live in shared memory and are
  never copied between processes.
- **Named streams** – processes find each other by stream name; no broker or
  daemon required.
- **Frame pooling** – released frames go back to a return queue and are reused,
  avoiding repeated shared memory allocations.
- **Blocking and async APIs** – blocking calls with timeouts, plus an
  `AsyncStream` that runs operations on a worker thread and reports back through
  callbacks.
- **Interruptible** – blocking operations can be interrupted via a controller
  or a periodically polled callback.
- **Clean shutdown** – closing a stream signals end-of-stream to consumers, and
  abandoned shared memory segments can be found and cleaned up.

## Crates

All crates live under [`rust/`](rust/).

| Crate | Description |
| --- | --- |
| [`bifrost`](rust/bifrost) | General purpose shared memory primitives, independent of media: named shared memory segments (`Shm`), the `MemoryMappable` trait and derive macro for laying out structs in shared memory, cross-process `Mutex` and `Event` primitives, and shared memory queues (`NameQueue`, `Queue`). |
| [`bifrost-frames`](rust/bifrost-frames) | The frame transport built on top of `bifrost`: `Stream` / `AsyncStream`, `Frame` with typed video and audio buffers, and frame metadata (`FrameInfo`, resolution, pixel format, sample rate, ...). |
| [`bifrost-frames-c`](rust/bifrost-frames-c) | C bindings for `bifrost-frames`, built as a shared library with a header ([`bifrost_frames.h`](rust/bifrost-frames-c/include/bifrost_frames.h)) and a pkg-config file. |

`bifrost/memory-mappable-derive` is the proc-macro crate behind
`#[derive(MemoryMappable)]` and is re-exported from `bifrost`.

## Example: streaming frames

```rust
use std::time::Duration;
use bifrost_frames::Stream;

let timeout = Duration::from_secs(5);

// Producer
let mut stream = Stream::new("my-stream", 5, 10)?; // queue size, pool size
let mut frame = stream.allocate(frame_info)?;
{
  let mut lock = frame.lock_mut(timeout)?;
  lock.set_frame_number(0);
  // ... write video/audio into the frame ...
}
stream.push(&mut Some(frame), timeout)?;

// Consumer (in another process)
let mut stream = Stream::open("my-stream")?;
while let Some(mut frame) = stream.pop(timeout)? {
  let lock = frame.lock(timeout)?;
  // ... read video/audio from the frame ...
  drop(lock);
  stream.release_frame(frame)?;
}
```

Runnable examples are in [`rust/bifrost-frames/examples`](rust/bifrost-frames/examples):

```sh
cd rust
cargo run --example create_stream -- my-stream
cargo run --example producer -- my-stream
cargo run --example consumer -- my-stream
```

## Building

```sh
cd rust
./build.sh          # cargo build + tests, plus the C API tests
./docker_build.sh   # same, inside a pinned Rust container
```

To install the C library, header and pkg-config file system-wide, run
`rust/bifrost-frames-c/install.sh` after a release build.

## Platform support

Linux is currently supported. Windows support is in progress on the
`experimental/windows-support` branch.

## License

MIT – see [LICENSE](LICENSE).
