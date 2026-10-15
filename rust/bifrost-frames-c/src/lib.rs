use std::{
  ffi::{
    CStr,
    c_void,
  },
  ptr,
  time::Duration,
};

#[repr(u32)]
#[derive(Debug)]
enum BifrostStatus {
  Ok = 0,
  InvalidArgument = 1,
  Utf8Error = 2,
  ShmError = 3,
  BadEnumValue = 4,
  NotFound = 5,
  AlreadyExists = 6,
  NotAFrame = 7,
  NotAFrameQueue = 8,
  NotAStream = 9,
  Timeout = 10,
  QueueClosed = 11,
  StreamFormatChange = 12,
  CorruptStream = 13,
  BadResolution = 14,
  BadFrameRate = 15,
  BadSampleRate = 16,
  Interrupted = 17,
  AsyncStreamThreadDead = 18,
}

impl BifrostStatus {
  fn from_u32(value: u32) -> Option<Self> {
    Some(match value {
      0 => BifrostStatus::Ok,
      1 => BifrostStatus::InvalidArgument,
      2 => BifrostStatus::Utf8Error,
      3 => BifrostStatus::ShmError,
      4 => BifrostStatus::BadEnumValue,
      5 => BifrostStatus::NotFound,
      6 => BifrostStatus::AlreadyExists,
      7 => BifrostStatus::NotAFrame,
      8 => BifrostStatus::NotAFrameQueue,
      9 => BifrostStatus::NotAStream,
      10 => BifrostStatus::Timeout,
      11 => BifrostStatus::QueueClosed,
      12 => BifrostStatus::StreamFormatChange,
      13 => BifrostStatus::CorruptStream,
      14 => BifrostStatus::BadResolution,
      15 => BifrostStatus::BadFrameRate,
      16 => BifrostStatus::BadSampleRate,
      17 => BifrostStatus::Interrupted,
      18 => BifrostStatus::AsyncStreamThreadDead,
      _ => {
        return None;
      }
    })
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_status_to_string(in_status: u32) -> *const i8 {
  let Some(status) = BifrostStatus::from_u32(in_status) else {
    return c"<invalid status".as_ptr();
  };

  let string : &'static CStr = match status {
    BifrostStatus::Ok => c"OK",
    BifrostStatus::InvalidArgument => c"Invalid argument",
    BifrostStatus::Utf8Error => c"Utf8 error",
    BifrostStatus::ShmError => c"Shared memory error",
    BifrostStatus::BadEnumValue => c"Bad enum value",
    BifrostStatus::NotFound => c"Not found",
    BifrostStatus::AlreadyExists => c"Already exists",
    BifrostStatus::NotAFrame => c"Not a frame",
    BifrostStatus::NotAFrameQueue => c"Not a frame queue",
    BifrostStatus::NotAStream => c"Not a stream",
    BifrostStatus::Timeout => c"Timeout",
    BifrostStatus::QueueClosed => c"Queue closed",
    BifrostStatus::StreamFormatChange => c"Stream format change",
    BifrostStatus::CorruptStream => c"Corrupt stream",
    BifrostStatus::BadResolution => c"Bad resolution",
    BifrostStatus::BadFrameRate => c"Bad frame rate",
    BifrostStatus::BadSampleRate => c"Bad sample rate",
    BifrostStatus::Interrupted => c"Interrupted",
    BifrostStatus::AsyncStreamThreadDead => c"Async stream thread died",
  };

  string.as_ptr()
}

impl From<bifrost_frames::Error> for BifrostStatus {
  fn from(value: bifrost_frames::Error) -> Self {
    use bifrost_frames::Error::*;
    match value {
      ShmError(..) => BifrostStatus::ShmError,
      BadEnumValue => BifrostStatus::BadEnumValue,
      NotFound => BifrostStatus::NotFound,
      AlreadyExists => BifrostStatus::AlreadyExists,
      NotAFrame => BifrostStatus::NotAFrame,
      NotAFrameQueue => BifrostStatus::NotAFrameQueue,
      NotAStream => BifrostStatus::NotAStream,
      Timeout => BifrostStatus::Timeout,
      QueueClosed => BifrostStatus::QueueClosed,
      StreamFormatChange => BifrostStatus::StreamFormatChange,
      CorruptStream => BifrostStatus::CorruptStream,
      BadResolution(..) => BifrostStatus::BadResolution,
      BadFrameRate(..) => BifrostStatus::BadFrameRate,
      BadSampleRate(..) => BifrostStatus::BadSampleRate,
      Interrupted => BifrostStatus::Interrupted,
      AsyncStreamThreadDead => BifrostStatus::AsyncStreamThreadDead,
    }
  }
}

unsafe fn c_string_to_string<'t>(c_str: *const i8) -> Result<&'t str, BifrostStatus> {
  unsafe { CStr::from_ptr(c_str) }.to_str()
    .map_err(|_| BifrostStatus::Utf8Error)
}

fn to_status_code<F: FnOnce() -> Result<(), BifrostStatus>>(f: F) -> BifrostStatus {
  match f() {
    Ok(()) => BifrostStatus::Ok,
    Err(error) => error,
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_create(
  name: *const i8,
  queue_size: usize,
  pool_size: usize,
  out_stream: *mut *mut bifrost_frames::Stream,
) -> BifrostStatus {
  if name == ptr::null() {
    return BifrostStatus::InvalidArgument;
  }
  if queue_size == 0 {
    return BifrostStatus::InvalidArgument;
  }
  if pool_size == 0 {
    return BifrostStatus::InvalidArgument;
  }
  if out_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if unsafe { *out_stream } != ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let name = unsafe { c_string_to_string(name)? };
    let stream = bifrost_frames::Stream::new(name, queue_size, pool_size)?;
    unsafe { *out_stream = Box::leak(Box::new(stream)) };
    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_open(
  name: *const i8,
  out_stream: *mut *mut bifrost_frames::Stream,
) -> BifrostStatus {
  if name == ptr::null() {
    return BifrostStatus::InvalidArgument;
  }
  if out_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if unsafe { *out_stream } != ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let name = unsafe { c_string_to_string(name)? };
    let stream = bifrost_frames::Stream::open(name)?;
    unsafe { *out_stream = Box::leak(Box::new(stream)) };
    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_destroy(
  inout_stream: *mut *mut bifrost_frames::Stream,
) -> BifrostStatus {
  if inout_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  unsafe {
    if  *inout_stream != ptr::null_mut() {
      let _ = Box::from_raw(*inout_stream);
      *inout_stream = ptr::null_mut();
    }
  }
  BifrostStatus::Ok
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_close(
  in_stream: *mut bifrost_frames::Stream,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  to_status_code(move || {
    let stream : &mut bifrost_frames::Stream = unsafe { &mut *in_stream };
    stream.close()?;
    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_push(
  in_stream: *mut bifrost_frames::Stream,
  timeout_ms: u64,
  inout_frame: *mut *mut bifrost_frames::Frame,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if inout_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if unsafe { *inout_frame } == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let stream : &mut bifrost_frames::Stream = unsafe { &mut *in_stream };

    let frame = unsafe { Box::from_raw(*inout_frame) };
    unsafe { *inout_frame = ptr::null_mut(); }

    stream.push(&mut Some(*frame), Duration::from_millis(timeout_ms))?;

    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_pop(
  in_stream: *mut bifrost_frames::Stream,
  timeout_ms: u64,
  out_frame: *mut *mut bifrost_frames::Frame,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if out_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if unsafe { *out_frame } != ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let stream : &mut bifrost_frames::Stream = unsafe { &mut *in_stream };

    match stream.pop(Duration::from_millis(timeout_ms))? {
      Some(frame) => {
        unsafe { *out_frame = Box::leak(Box::new(frame)) };
        Ok(())
      },
      None => {
        // End of stream is signaled by bifrost_stream_pop returning ok status,
        // but out_frame is NULL.
        Ok(())
      }
    }
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_wait_until_drained(
  in_stream: *mut bifrost_frames::Stream,
  timeout_ms: u64,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  to_status_code(move || {
    let in_stream = unsafe { &mut *in_stream };
    in_stream.wait_until_drained(Duration::from_millis(timeout_ms))?;
    Ok(())
  })
}

struct CInterruptCallback {
  callback: unsafe extern "C" fn(*mut c_void) -> u32,
  userdata: *mut c_void,
}

impl CInterruptCallback {
  fn call(&mut self) -> bool {
    unsafe { (self.callback)(self.userdata)  != 0 }
  }
}

unsafe impl Send for CInterruptCallback { }

#[unsafe(no_mangle)]
unsafe extern "C" fn  bifrost_stream_register_interrupt_callback(
  in_stream: *mut bifrost_frames::Stream,
  callback: Option<unsafe extern "C" fn(*mut c_void) -> u32>,
  userdata: *mut c_void,
  interval_ms: u64,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let Some(callback) = callback else {
    return BifrostStatus::InvalidArgument;
  };

  let mut c_callback = CInterruptCallback { callback, userdata };

  let in_stream = unsafe { &mut *in_stream };
  in_stream.register_interrupt_callback(
    move || { c_callback.call() },
    Duration::from_millis(interval_ms),
  );

  return BifrostStatus::Ok;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_clear_interrupt_callback(
  in_stream: *mut bifrost_frames::Stream,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  let in_stream = unsafe { &mut *in_stream };
  in_stream.clear_interrupt_callback();
  return BifrostStatus::Ok;
}

#[repr(C)]
struct BifrostVideoInfo {
  resolution: bifrost_frames::Resolution,
  pixel_format: u32,
  field_order: u32,
}

#[repr(C)]
struct BifrostAudioInfo {
  sample_rate: u32,
  channel_count: u32,
  format: u32,
}

#[repr(C)]
struct BifrostFrameInfo {
  frame_rate: bifrost_frames::Fraction,
  video: *const BifrostVideoInfo,
  audio: *const BifrostAudioInfo,
}

impl BifrostFrameInfo {
  unsafe fn to_frame_info(self) -> Result<bifrost_frames::FrameInfo, bifrost_frames::Error> {
    let frame_info = bifrost_frames::FrameInfo {
      frame_rate: bifrost_frames::Fraction {
        numerator: self.frame_rate.numerator,
        denominator: self.frame_rate.denominator,
      },
      video: if self.video != ptr::null() {
        let in_video : BifrostVideoInfo = unsafe { ptr::read_unaligned(self.video) };

        Some(bifrost_frames::VideoInfo {
          resolution: in_video.resolution,
          field_order: bifrost_frames::FieldOrder::from_u32(in_video.field_order)?,
          pixel_format: bifrost_frames::PixelFormat::from_u32(in_video.pixel_format)?,
        })
      } else {
        None
      },
      audio: if self.audio != ptr::null() {
        let in_audio : BifrostAudioInfo = unsafe { ptr::read_unaligned(self.audio) };

        Some(bifrost_frames::AudioInfo {
          sample_rate: in_audio.sample_rate,
          channel_count: in_audio.channel_count,
          format: bifrost_frames::AudioFormat::from_u32(in_audio.format)?,
        })
      } else {
        None
      },
    };

    Ok(frame_info)
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_stream_allocate_frame(
  in_stream: *mut bifrost_frames::Stream,
  in_frame_info: *const BifrostFrameInfo,
  out_frame: *mut *mut bifrost_frames::Frame,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if in_frame_info == ptr::null() {
    return BifrostStatus::InvalidArgument;
  }

  if out_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if unsafe { *out_frame } != ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let stream : &mut bifrost_frames::Stream = unsafe { &mut *in_stream };

    let in_frame_info : BifrostFrameInfo = unsafe { ptr::read_unaligned(in_frame_info) };

    let frame_info = unsafe { in_frame_info.to_frame_info()? };

    let frame = stream.allocate(frame_info)?;

    unsafe { *out_frame = Box::leak(Box::new(frame)) }

    Ok(())
  })
}


#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_release(
  in_stream: *mut bifrost_frames::Stream,
  inout_frame: *mut *mut bifrost_frames::Frame,
) -> BifrostStatus {
  if inout_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if unsafe { *inout_frame } == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }


  to_status_code(move || {
    let frame = unsafe { Box::from_raw(*inout_frame) };
    unsafe { *inout_frame = ptr::null_mut(); }

    if in_stream != ptr::null_mut() {
      let stream : &mut bifrost_frames::Stream = unsafe { &mut *in_stream };
      stream.release_frame(*frame)?;
    }

    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_destroy(
  inout_frame: *mut *mut bifrost_frames::Frame,
) -> BifrostStatus {
  if inout_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if unsafe { *inout_frame } == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }


  to_status_code(move || {
    let _ = unsafe { Box::from_raw(*inout_frame) };
    Ok(())
  })
}

#[repr(C)]
struct FrameQueryResult {
  frame_rate: bifrost_frames::Fraction,
  has_video: u32,
  has_audio: u32,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_query(
  in_frame: *mut bifrost_frames::Frame,
  mut_frame_query_result: *mut FrameQueryResult,
) -> BifrostStatus {
  if in_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if mut_frame_query_result == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let frame : &mut bifrost_frames::Frame = unsafe { &mut *in_frame };

    let frame_info = frame.info()?;
    let result = FrameQueryResult {
      frame_rate: frame_info.frame_rate,
      has_video: if frame_info.video.is_some() { 1 } else { 0 },
      has_audio: if frame_info.audio.is_some() { 1 } else { 0 },
    };

    unsafe { mut_frame_query_result.write(result) };

    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_query_video_info(
  in_frame: *mut bifrost_frames::Frame,
  mut_video_info: *mut BifrostVideoInfo,
) -> BifrostStatus {
  if in_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if mut_video_info == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let frame : &mut bifrost_frames::Frame = unsafe { &mut *in_frame };
  let frame_info = match frame.info() {
    Ok(frame_info) => frame_info,
    Err(error) => return error.into(),
  };

  let Some(video_info) = frame_info.video else {
    return BifrostStatus::InvalidArgument;
  };

  let out_video_info = BifrostVideoInfo {
    resolution: video_info.resolution,
    pixel_format: video_info.pixel_format.to_u32(),
    field_order: video_info.field_order.to_u32(),
  };

  unsafe { mut_video_info.write(out_video_info) };

  return BifrostStatus::Ok;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_query_audio_info(
  in_frame: *mut bifrost_frames::Frame,
  mut_audio_info: *mut BifrostAudioInfo,
) -> BifrostStatus {
  if in_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if mut_audio_info == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let frame : &mut bifrost_frames::Frame = unsafe { &mut *in_frame };
  let frame_info = match frame.info() {
    Ok(frame_info) => frame_info,
    Err(error) => return error.into(),
  };

  let Some(audio_info) = frame_info.audio else {
    return BifrostStatus::InvalidArgument;
  };

  let out_audio_info = BifrostAudioInfo {
    sample_rate: audio_info.sample_rate,
    channel_count: audio_info.channel_count,
    format: audio_info.format.to_u32(),
  };

  unsafe { mut_audio_info.write(out_audio_info) };

  return BifrostStatus::Ok;
}

/**
 *  The bifrost stream is designed so that in practice only one process should
 *  hold a frame at the one time. The frame ownership is moved by pushing onto
 *  the queue and returning the frame. Because of this the frame locking
 *  timeout is purely a fail safe to avoid processes hanging.
 */
const LOCK_TIMEOUT : Duration = Duration::from_secs(5);

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_lock(
  in_frame: *mut bifrost_frames::Frame,
  out_lock: *mut *mut bifrost_frames::FrameLockMut<'static>,
) -> BifrostStatus {
  if in_frame == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if out_lock == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if unsafe { *out_lock } != ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let frame : &mut bifrost_frames::Frame = unsafe { &mut *in_frame };
    let lock = frame.lock_mut(LOCK_TIMEOUT)?;
    unsafe { *out_lock = Box::leak(Box::new(lock)) }
    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_unlock(
  inout_lock: *mut *mut bifrost_frames::FrameLockMut<'static>,
) -> BifrostStatus {
  if inout_lock == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  unsafe {
    if *inout_lock != ptr::null_mut() {
      let _ = Box::from_raw(*inout_lock);
      *inout_lock = ptr::null_mut();
    }
    return BifrostStatus::Ok;
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_lock_get_frame_number(
  in_lock: *mut bifrost_frames::FrameLockMut<'static>,
  mut_frame_number: *mut u64,
) -> BifrostStatus {
  if in_lock == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if mut_frame_number == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let in_lock = unsafe { &mut *in_lock };
  unsafe { mut_frame_number.write(in_lock.frame_number()) }
  return BifrostStatus::Ok;
 }

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_lock_set_frame_number(
  in_lock: *mut bifrost_frames::FrameLockMut<'static>,
  frame_number: u64,
) -> BifrostStatus {
  if in_lock == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  let in_lock = unsafe { &mut *in_lock };
  in_lock.set_frame_number(frame_number);
  return BifrostStatus::Ok;
}

#[repr(C)]
struct BifrostVideoBuffer {
  ptr: *mut u8,
  size: usize,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_lock_query_video_buffer(
  in_lock: *mut bifrost_frames::FrameLockMut<'static>,
  mut_buffer: *mut BifrostVideoBuffer,
) -> BifrostStatus {
  if in_lock == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if mut_buffer == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let in_lock = unsafe { &mut *in_lock };
  let video = match in_lock.video() {
    Ok(Some(video)) => video,
    Ok(None) => {
      return BifrostStatus::InvalidArgument;
    },
    Err(error) => {
      return error.into();
    }
  };

  let out_buffer = BifrostVideoBuffer {
    ptr: video.buffer.as_mut_ptr(),
    size: video.buffer.len(),
  };

  unsafe { mut_buffer.write(out_buffer); }

  return BifrostStatus::Ok;
}

#[repr(C)]
struct BifrostAudioBuffer {
  ptr: *mut u8,
  size: usize,
  sample_count: u64,
  sample_offset: u64,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_frame_lock_query_audio_buffer(
  in_lock: *mut bifrost_frames::FrameLockMut<'static>,
  mut_buffer: *mut BifrostAudioBuffer,
) -> BifrostStatus {
  if in_lock == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if mut_buffer == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let in_lock = unsafe { &mut *in_lock };
  let audio = match in_lock.audio() {
    Ok(Some(audio)) => audio,
    Ok(None) => {
      return BifrostStatus::InvalidArgument;
    },
    Err(error) => {
      return error.into();
    }
  };

  let out_buffer = BifrostAudioBuffer {
    ptr: audio.buffer.as_mut_ptr(),
    size: audio.buffer.len(),
    sample_count: audio.sample_count,
    sample_offset: audio.sample_offset,
  };

  unsafe { mut_buffer.write(out_buffer); }

  return BifrostStatus::Ok;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_async_stream_create(
  name: *const i8,
  queue_size: usize,
  pool_size: usize,
  out_stream: *mut *mut bifrost_frames::AsyncStream,
) -> BifrostStatus {
  if name == ptr::null() {
    return BifrostStatus::InvalidArgument;
  }
  if queue_size == 0 {
    return BifrostStatus::InvalidArgument;
  }
  if pool_size == 0 {
    return BifrostStatus::InvalidArgument;
  }
  if out_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if unsafe { *out_stream } != ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let name = unsafe { c_string_to_string(name)? };
    let stream = bifrost_frames::Stream::new(name, queue_size, pool_size)?;
    let async_stream = bifrost_frames::AsyncStream::new(stream)?;
    unsafe { *out_stream = Box::leak(Box::new(async_stream)) };
    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_async_stream_open(
  name: *const i8,
  out_stream: *mut *mut bifrost_frames::AsyncStream,
) -> BifrostStatus {
  if name == ptr::null() {
    return BifrostStatus::InvalidArgument;
  }
  if out_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  if unsafe { *out_stream } != ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  to_status_code(move || {
    let name = unsafe { c_string_to_string(name)? };
    let stream = bifrost_frames::Stream::open(name)?;
    let async_stream = bifrost_frames::AsyncStream::new(stream)?;
    unsafe { *out_stream = Box::leak(Box::new(async_stream)) };
    Ok(())
  })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_async_stream_destroy(
  inout_stream: *mut *mut bifrost_frames::AsyncStream,
) -> BifrostStatus {
  if inout_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }
  unsafe {
    if  *inout_stream != ptr::null_mut() {
      let _ = Box::from_raw(*inout_stream);
      *inout_stream = ptr::null_mut();
    }
  }
  BifrostStatus::Ok
}

struct PushCallback {
  callback: extern "C" fn (*mut c_void, BifrostStatus) -> (),
  user_data: *mut c_void,
}

impl PushCallback {
  fn call(self, status: BifrostStatus) {
    (self.callback)(self.user_data, status);
  }
}

unsafe impl Send for PushCallback { }

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_async_stream_push(
  in_stream: *mut bifrost_frames::AsyncStream,
  timeout_ms: u64,
  inout_frame: *mut *mut bifrost_frames::Frame,
  callback: Option<extern "C" fn (*mut c_void, BifrostStatus) -> ()>,
  user_data: *mut c_void,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let Some(callback) = callback else {
    return BifrostStatus::InvalidArgument;
  };

  to_status_code(move || {
    let stream : &mut bifrost_frames::AsyncStream = unsafe { &mut *in_stream };

    let frame = unsafe { Box::from_raw(*inout_frame) };
    unsafe { *inout_frame = ptr::null_mut(); }

    let callback = PushCallback { callback, user_data };

    stream.push(
      *frame,
      Duration::from_millis(timeout_ms),
      move |result| {
        match result {
          Ok(()) => callback.call(BifrostStatus::Ok),
          Err(error) => callback.call(error.into()),
        }
      }
    )?;

    Ok(())
  })
}

struct PopCallback {
  callback: extern "C" fn (*mut c_void, BifrostStatus, *mut bifrost_frames::Frame),
  user_data: *mut c_void
}

impl PopCallback {
  fn call(
    self,
    result: Result<Option<bifrost_frames::Frame>, bifrost_frames::Error>
  ) {
    match result {
      Ok(Some(frame)) => {
        (self.callback)(self.user_data, BifrostStatus::Ok, Box::leak(Box::new(frame)));
      },
      Ok(None) => {
        (self.callback)(self.user_data, BifrostStatus::Ok, ptr::null_mut());
      },
      Err(error) => {
        (self.callback)(self.user_data, error.into(), ptr::null_mut());
      }
    }
  }
}

unsafe impl Send for PopCallback { }

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_async_stream_pop(
  in_stream: *mut bifrost_frames::AsyncStream,
  timeout_ms: u64,
  callback: Option<extern "C" fn (*mut c_void, BifrostStatus, *mut bifrost_frames::Frame)>,
  user_data: *mut c_void
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let Some(callback) = callback else {
    return BifrostStatus::InvalidArgument;
  };

  to_status_code(move || {
    let stream : &mut bifrost_frames::AsyncStream = unsafe { &mut *in_stream };

    let pop_callback = PopCallback { callback, user_data };

    stream.pop(
      Duration::from_millis(timeout_ms),
      move |result| { pop_callback.call(result); }
    )?;

    Ok(())
  })
}

struct WaitUntilDrainedCallback {
  callback: extern "C" fn (*mut c_void, BifrostStatus) -> (),
  user_data: *mut c_void,
}

impl WaitUntilDrainedCallback {
  fn call(self, result: Result<(), bifrost_frames::Error>) {
    match result {
      Ok(()) => (self.callback)(self.user_data, BifrostStatus::Ok),
      Err(error) => (self.callback)(self.user_data, error.into()),
    }
  }
}

unsafe impl Send for WaitUntilDrainedCallback { }

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_async_stream_wait_until_drained(
  in_stream: *mut bifrost_frames::AsyncStream,
  timeout_ms: u64,
  callback: Option<extern "C" fn (*mut c_void, BifrostStatus) -> ()>,
  user_data: *mut c_void,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let Some(callback) = callback else {
    return BifrostStatus::InvalidArgument;
  };

  to_status_code(move || {
    let in_stream = unsafe { &mut *in_stream };

    let callback = WaitUntilDrainedCallback { callback, user_data };

    in_stream.wait_until_drained(
      Duration::from_millis(timeout_ms),
      move |result| { callback.call(result) }
    )?;
    Ok(())
  })
}

struct AllocateFrameCallback {
  callback: extern "C" fn (*mut c_void, BifrostStatus, *mut bifrost_frames::Frame),
  user_data: *mut c_void
}

impl AllocateFrameCallback {
  fn call(self, result: Result<bifrost_frames::Frame, bifrost_frames::Error>) {
    match result {
      Ok(frame) => {
        (self.callback)(self.user_data, BifrostStatus::Ok, Box::leak(Box::new(frame)));
      },
      Err(error) => {
        (self.callback)(self.user_data, error.into(), ptr::null_mut());
      }
    }
  }
}

unsafe impl Send for AllocateFrameCallback { }

#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_async_stream_allocate_frame(
  in_stream: *mut bifrost_frames::AsyncStream,
  in_frame_info: *const BifrostFrameInfo,
  callback: Option<extern "C" fn (*mut c_void, BifrostStatus, *mut bifrost_frames::Frame)>,
  user_data: *mut c_void
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  if in_frame_info == ptr::null() {
    return BifrostStatus::InvalidArgument;
  }

  let Some(callback) = callback else {
    return BifrostStatus::InvalidArgument;
  };

  to_status_code(move || {
    let stream : &mut bifrost_frames::AsyncStream = unsafe { &mut *in_stream };
    let in_frame_info : BifrostFrameInfo = unsafe { ptr::read_unaligned(in_frame_info) };

    let allocate_frame_callback = AllocateFrameCallback { callback, user_data };

    let frame_info = unsafe { in_frame_info.to_frame_info()? };

    stream.allocate(
      frame_info,
      move |result| { allocate_frame_callback.call(result); }
    )?;

    Ok(())
  })
}

struct ReleaseFrameCallback {
  callback: extern "C" fn (*mut c_void, BifrostStatus) -> (),
  user_data: *mut c_void,
}

impl ReleaseFrameCallback {
  fn call(self, status: BifrostStatus) {
    (self.callback)(self.user_data, status);
  }
}

unsafe impl Send for ReleaseFrameCallback { }


#[unsafe(no_mangle)]
unsafe extern "C" fn bifrost_async_stream_release_frame(
  in_stream: *mut bifrost_frames::AsyncStream,
  inout_frame: *mut *mut bifrost_frames::Frame,
  callback: Option<extern "C" fn (*mut c_void, BifrostStatus) -> ()>,
  user_data: *mut c_void,
) -> BifrostStatus {
  if in_stream == ptr::null_mut() {
    return BifrostStatus::InvalidArgument;
  }

  let Some(callback) = callback else {
    return BifrostStatus::InvalidArgument;
  };

  to_status_code(move || {
    let frame = unsafe { Box::from_raw(*inout_frame) };
    unsafe { *inout_frame = ptr::null_mut(); }

    let stream : &mut bifrost_frames::AsyncStream = unsafe { &mut *in_stream };

    let callback = ReleaseFrameCallback { callback, user_data };

    stream.release_frame(
      *frame,
      move |result| {
        match result {
          Ok(()) => callback.call(BifrostStatus::Ok),
          Err(error) => callback.call(error.into()),
        }
      }
    )?;

    Ok(())
  })
}
