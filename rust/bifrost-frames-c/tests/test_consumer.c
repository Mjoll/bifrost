#include "bifrost_frames.h"
#include "common.h"

int main() {
  int ret = 1;

  BifrostStream stream = NULL;
  BifrostFrame frame = NULL;
  BifrostFrameLock lock = NULL;

  BIFROST_CHECK(bifrost_stream_open(TEST_QUEUE_NAME, &stream));

  uint64_t timeout_ms = 5000;

  uint64_t expected_frame_number = 0;
  uint64_t expected_sample_offset = 0;
  while (1) {
    BIFROST_CHECK(bifrost_stream_pop(stream, timeout_ms, &frame));

    if (frame == NULL) {
      break;
    }

    struct BifrostFrameQueryResult frame_query_result = { 0 };

    BIFROST_CHECK(bifrost_frame_query(frame, &frame_query_result));

    ASSERT_EQUAL(frame_query_result.frame_rate.numerator, test_frame_info.frame_rate.numerator);
    ASSERT_EQUAL(frame_query_result.frame_rate.denominator, test_frame_info.frame_rate.denominator);

    struct BifrostVideoInfo video_info = { 0 };
    BIFROST_CHECK(bifrost_frame_query_video_info(frame, &video_info));
    ASSERT_EQUAL(video_info.resolution.width, test_video_info.resolution.width);
    ASSERT_EQUAL(video_info.resolution.height, test_video_info.resolution.height);
    ASSERT_EQUAL(video_info.resolution.pixel_aspect_ratio.numerator, test_video_info.resolution.pixel_aspect_ratio.numerator);
    ASSERT_EQUAL(video_info.resolution.pixel_aspect_ratio.denominator, test_video_info.resolution.pixel_aspect_ratio.denominator);
    ASSERT_EQUAL(video_info.pixel_format, test_video_info.pixel_format);
    ASSERT_EQUAL(video_info.field_order, test_video_info.field_order);

    struct BifrostAudioInfo audio_info = { 0 };
    BIFROST_CHECK(bifrost_frame_query_audio_info(frame, &audio_info));
    ASSERT_EQUAL(audio_info.sample_rate, test_audio_info.sample_rate);
    ASSERT_EQUAL(audio_info.channel_count, test_audio_info.channel_count);
    ASSERT_EQUAL(audio_info.format, test_audio_info.format);

    BIFROST_CHECK(bifrost_frame_lock(frame, &lock));

    uint64_t frame_number = 0;
    BIFROST_CHECK(bifrost_frame_lock_get_frame_number(lock, &frame_number));
    ASSERT_EQUAL(frame_number, expected_frame_number);

    struct BifrostVideoBuffer video_buffer = { 0 };
    BIFROST_CHECK(bifrost_frame_lock_query_video_buffer(lock, &video_buffer));

    uint8_t expected_video_value_byte = 32 + (frame_number % 32);
    uint64_t expected_video_value = 0;
    for (int i = 0; i < 8; i++) {
      expected_video_value = expected_video_value << 8;
      expected_video_value |= (uint64_t)expected_video_value_byte;
    }
    const uint64_t* src = (uint64_t*)video_buffer.ptr;
    for (size_t i = 0; i < video_buffer.size / 8; i++) {
      uint64_t value = *(src++);
      if (value != expected_video_value) {
        ASSERT_EQUAL(value, expected_video_value);
      }
    }

    struct BifrostAudioBuffer audio_buffer = { 0 };
    BIFROST_CHECK(bifrost_frame_lock_query_audio_buffer(lock, &audio_buffer));

    // Note: This check only works because the sample count is the same for each frame
    uint64_t expected_sample_count = (test_audio_info.sample_rate * test_frame_info.frame_rate.denominator) / test_frame_info.frame_rate.numerator;

    ASSERT_EQUAL(audio_buffer.sample_count, expected_sample_count);

    ASSERT_EQUAL(audio_buffer.sample_offset, expected_sample_offset);
    expected_sample_offset += audio_buffer.sample_count;

    uint8_t expected_audio_value = 64 + (frame_number % 32);
    for (size_t i = 0; i < audio_buffer.size; i++) {
      ASSERT_EQUAL(audio_buffer.ptr[i], expected_audio_value);
    }

    BIFROST_CHECK(bifrost_frame_unlock(&lock));

    BIFROST_CHECK(bifrost_frame_release(stream, &frame));

    expected_frame_number ++;
  }

  ASSERT_EQUAL(expected_frame_number, TEST_FRAME_COUNT);

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
