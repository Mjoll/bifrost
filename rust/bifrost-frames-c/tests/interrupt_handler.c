#include <unistd.h>
#include <signal.h>
#include <stdio.h>

#include "interrupt_handler.h"
#include "bifrost_frames.h"
#include "common.h"

static uint32_t signal_count = 0;

static void handle_sigint(int _signal) {
  signal_count ++;
  if (signal_count >= 2) {
    signal(SIGINT, SIG_DFL);
    raise(SIGINT);
  }
  fprintf(stderr, "Caught first SIGINT\n");
}

static uint32_t is_interrupted(void* userdata) {
  return (signal_count != 0) ? 1 : 0;
}

void setup_interrupt_handler(BifrostStream stream) {
  BIFROST_CHECK(bifrost_stream_register_interrupt_callback(stream, is_interrupted, NULL, 100));
  signal(SIGINT, handle_sigint);
  return;
fail:
  fprintf(stderr, "Failed to register interrupt callback");
}

void tear_down_interrupt_handler(BifrostStream stream) {
  signal(SIGINT, SIG_DFL);
  BIFROST_CHECK(bifrost_stream_clear_interrupt_callback(stream));
  return;
fail:
  fprintf(stderr, "Failed to unregister interrupt callback");
  return;
}
