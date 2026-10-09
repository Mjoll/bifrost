#include <string.h>
#include "bifrost_frames.h"
#include "common.h"

int main() {
  int ret = 1;
  BifrostStream stream = NULL;
  BifrostFrame frame = NULL;
  BifrostFrameLock lock = NULL;

  BIFROST_CHECK(bifrost_stream_create(TEST_QUEUE_NAME, 5, 10, &stream));

  const size_t timeout_ms = 5000;

  for (uint64_t frame_number = 0; frame_number < TEST_FRAME_COUNT; frame_number ++) {
    fprintf(stderr, "Allocating frame %ld\n", frame_number);
    BIFROST_CHECK(bifrost_stream_allocate_frame(stream, &test_frame_info, &frame));

    BIFROST_CHECK(bifrost_frame_lock(frame, &lock));

    BIFROST_CHECK(bifrost_frame_lock_set_frame_number(lock, frame_number));

    struct BifrostVideoBuffer video_buffer = { 0 };

    BIFROST_CHECK(bifrost_frame_lock_query_video_buffer(lock, &video_buffer));

    memset(video_buffer.ptr, 32 + (frame_number % 32), video_buffer.size);

    struct BifrostAudioBuffer audio_buffer = { 0 };

    BIFROST_CHECK(bifrost_frame_lock_query_audio_buffer(lock, &audio_buffer));

    memset(audio_buffer.ptr, 64 + (frame_number % 32), audio_buffer.size);

    BIFROST_CHECK(bifrost_frame_unlock(&lock));

    fprintf(stderr, "Pushing frame %ld\n", frame_number);
    BIFROST_CHECK(bifrost_stream_push(stream, timeout_ms, &frame));
  }

  fprintf(stderr, "Waiting for stream to be drained\n");
  BIFROST_CHECK(bifrost_stream_wait_until_drained(stream, timeout_ms));

  ret = 0;

fail:
  fprintf(stderr, "Performing clean up\n");
  if (lock != NULL) {
    bifrost_frame_unlock(&lock);
  }
  if (frame != NULL) {
    bifrost_frame_release(stream, &frame);
  }
  if (stream != NULL) {
    bifrost_stream_destroy(&stream);
  }

  fprintf(stderr, "Finished with exit code %d\n", ret);

  return ret;
}
