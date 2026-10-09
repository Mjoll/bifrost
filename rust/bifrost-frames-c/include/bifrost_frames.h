#ifndef BIFROST_H
#define BIFROST_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>

enum BifrostStatus {
  BIFROST_STATUS_OK = 0,
  BIFROST_STATUS_INVALID_ARGUMENT = 1,
  BIFROST_STATUS_UTF8_ERROR = 2,
  BIFROST_STATUS_SHM_ERROR = 3,
  BIFROST_STATUS_BAD_ENUM_VALUE = 4,
  BIFROST_STATUS_NOT_FOUND = 5,
  BIFROST_STATUS_ALREADY_EXISTS = 6,
  BIFROST_STATUS_NOT_A_FRAME = 7,
  BIFROST_STATUS_NOT_A_FRAME_QUEUE = 8,
  BIFROST_STATUS_NOT_A_STREAM = 9,
  BIFROST_STATUS_TIMEOUT = 10,
  BIFROST_STATUS_QUEUE_CLOSED = 11,
  BIFROST_STATUS_STREAM_FORMAT_CHANGE = 12,
  BIFROST_STATUS_CORRUPT_STREAM = 13,
  BIFROST_STATUS_BAD_RESOLUTION = 14,
  BIFROST_STATUS_BAD_FRAME_RATE = 15,
  BIFROST_STATUS_BAD_SAMPLE_RATE = 16,
  BIFROST_STATUS_INTERRUPTED = 17,
};

const char* bifrost_status_to_string(enum BifrostStatus status);

struct BifrostStreamST;
typedef struct BifrostStreamST* BifrostStream;

struct BifrostFrameST;
typedef struct BifrostFrameST* BifrostFrame;

struct BifrostFrameLockST;
typedef struct BifrostFrameLockST* BifrostFrameLock;

struct BifrostFraction {
  uint32_t numerator;
  uint32_t denominator;
};

struct BifrostResolution {
  uint32_t width;
  uint32_t height;
  struct BifrostFraction pixel_aspect_ratio;
};

enum BifrostPixelFormat {
  BIFROST_PIXEL_FORMAT_RGBA8 = 0,
  BIFROST_PIXEL_FORMAT_RGBA16 = 1,
};

enum BifrostFieldOrder {
  BIFROST_FIELD_ORDER_PROGRESSIVE = 0,
  BIFROST_FIELD_ORDER_TOP_FIELD_FIRST = 1,
  BIFROST_FIELD_ORDER_BOTTOM_FIELD_FIRST = 2,
};

struct BifrostVideoInfo {
  struct BifrostResolution resolution;
  enum BifrostPixelFormat pixel_format;
  enum BifrostFieldOrder field_order;
};

enum BifrostAudioFormat {
  BIFROST_AUDIO_FORMAT_S16 = 0,
  BIFROST_AUDIO_FORMAT_F32,
};

struct BifrostAudioInfo {
  uint32_t sample_rate;
  uint32_t channel_count;
  enum BifrostAudioFormat format;
};


struct BifrostFrameInfo {
  struct BifrostFraction frame_rate;
  const struct BifrostVideoInfo* video;
  const struct BifrostAudioInfo* audio;
};

enum BifrostStatus bifrost_stream_create(
  const char* name,
  size_t queue_size,
  size_t pool_size,
  BifrostStream* out_stream
);

enum BifrostStatus bifrost_stream_open(
  const char* name,
  BifrostStream* out_stream
);

/**
 *  Destroy the stream object. If the stream was created
 *  using bifrost_stream_create then the stream will be closed. When closing
 *  the stream all consumers will get all null frame on pop indicating it is
 *  the end of the stream. All consumers push will fail with a QUEUE_CLOSED
 *  error.
 */
enum BifrostStatus bifrost_stream_destroy(
  BifrostStream* inout_stream
);

/**
 *  Close the stream. This is a way for processes who did not create the
 *  stream to mark end of stream. Useful if the producer is not the creator
 *  of the stream. Note that the stream must also be destroyed using
 *  bifrost_stream_destroy after calling this function.
 */
enum BifrostStatus bifrost_stream_close(
  BifrostStream in_stream
);

enum BifrostStatus bifrost_stream_push(
  BifrostStream in_stream,
  uint64_t timeout_ms,
  BifrostFrame *inout_frame
);

enum BifrostStatus bifrost_stream_pop(
  BifrostStream in_stream,
  uint64_t timeout_ms,
  BifrostFrame *out_frame
);

enum BifrostStatus bifrost_stream_wait_until_drained(
  BifrostStream in_stream,
  uint64_t timeout_ms
);

/**
 *  Register a interrupt callback. Existing interrupt callback will be replaced.
 *
 *  The interrupt callback will be called with the given interval on a
 *  callback thread. If the callback returns a non-zero return value the
 *  process local state of the stream will be marked as interrupted. The
 *  callback thread will stop and any blocking calls in progress or future
 *  blocking calls will immediately return BIFROST_STATUS_INTERRUPTED. In
 *  order to clear this state call bifrost_stream_clear_interrupt_callback.
 *
 *  Important: Since the callback is called on a different thread it is
 *  important that this callback is thread safe.
 */
enum BifrostStatus bifrost_stream_register_interrupt_callback(
  BifrostStream in_stream,
  uint32_t (*callback)(void*),
  void* user_data,
  uint64_t check_interval_ms
);

/**
 *  Clear current registered interrupt callback
 */
enum BifrostStatus bifrost_stream_clear_interrupt_callback(
  BifrostStream in_stream
);

enum BifrostStatus bifrost_stream_allocate_frame(
  BifrostStream in_stream,
  const struct BifrostFrameInfo* in_frame_info,
  BifrostFrame* out_frame
);


/**
 *  Release a frame by returning it to the return queue of the provided stream.
 *  If stream is NULL the frame will be destroyed and the shared memory cleaned
 *  up.
 */
enum BifrostStatus bifrost_frame_release(
  BifrostStream in_stream,
  BifrostFrame* inout_frame
);

struct BifrostFrameQueryResult {
  struct BifrostFraction frame_rate;
  uint32_t has_video;
  uint32_t has_audio;
};

enum BifrostStatus bifrost_frame_query(
  BifrostFrame in_frame,
  struct BifrostFrameQueryResult* mut_frame_query_result
);

enum BifrostStatus bifrost_frame_query_video_info(
  BifrostFrame in_frame,
  struct BifrostVideoInfo* mut_video_info
);

enum BifrostStatus bifrost_frame_query_audio_info(
  BifrostFrame in_frame,
  struct BifrostAudioInfo* mut_audio_info
);

enum BifrostStatus bifrost_frame_lock(
  BifrostFrame in_frame,
  BifrostFrameLock* out_lock
);

enum BifrostStatus bifrost_frame_unlock(
  BifrostFrameLock* inout_lock
);

enum BifrostStatus bifrost_frame_lock_get_frame_number(
  BifrostFrameLock in_lock,
  uint64_t* out_frame_number
);

enum BifrostStatus bifrost_frame_lock_set_frame_number(
  BifrostFrameLock in_lock,
  uint64_t frame_number
);

struct BifrostVideoBuffer {
  uint8_t* ptr;
  size_t size;
};

enum BifrostStatus bifrost_frame_lock_query_video_buffer(
  BifrostFrameLock in_lock,
  struct BifrostVideoBuffer* mut_buffer
);

struct BifrostAudioBuffer {
  uint8_t* ptr;
  size_t size;
  /**
   * Number of samples per channel in this audio buffer.
   */
  uint64_t sample_count;

  /**
   * Offset from the first sample in frame number 0 in number of samples
   */
  uint64_t sample_offset;
};

enum BifrostStatus bifrost_frame_lock_query_audio_buffer(
  BifrostFrameLock in_lock,
  struct BifrostAudioBuffer* mut_buffer
);


#ifdef __cplusplus
} // extern "C"
#endif

#endif // BIFROST_H
