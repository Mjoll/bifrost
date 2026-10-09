#include <stddef.h>
#include <stdio.h>
#include <bifrost_frames.h>

#define TEST_QUEUE_NAME "bifrost_frames_c_test_0"
#define TEST_FRAME_COUNT 1000

const struct BifrostVideoInfo test_video_info = {
  .resolution = {
    .width = 1920,
    .height = 1080,
    .pixel_aspect_ratio = {
      .numerator = 1,
      .denominator = 1,
    }
  },
  .pixel_format = BIFROST_PIXEL_FORMAT_RGBA8,
  .field_order = BIFROST_FIELD_ORDER_PROGRESSIVE,
};

const struct BifrostAudioInfo test_audio_info = {
  .sample_rate = 48000,
  .channel_count = 2,
  .format = BIFROST_AUDIO_FORMAT_S16,
};

const struct BifrostFrameInfo test_frame_info = {
  .frame_rate = {
    .numerator = 25,
    .denominator = 1,
  },
  .video = &test_video_info,
  .audio = &test_audio_info,
};

#define BIFROST_CHECK(x) do { \
  int ret = x; \
  if (ret != BIFROST_STATUS_OK) { \
    fprintf(stderr, "ERROR: Bifrost call to " #x " failed with return code %d (%s)\n", ret, bifrost_status_to_string(ret)); \
    goto fail; \
  } \
} while (0);

#define ASSERT_EQUAL(a, b) do { \
  if (a != b) { \
    fprintf(stderr, "ERROR: Assertion failed: " #a " == " #b "(%ld != %ld)\n", (int64_t)a, (int64_t)b); \
    goto fail; \
  } \
} while (0);


