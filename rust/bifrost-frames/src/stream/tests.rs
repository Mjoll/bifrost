use std::{
  thread,
  time::Duration,
};

use crate::{
  AudioInfo,
  FieldOrder,
  Fraction,
  PixelFormat,
  Resolution,
  VideoInfo,
};

use super::{
  Error,
  FrameInfo,
  Stream,
};

#[test]
fn test_stream_1() {
  let stream_name = "bifrost-frames-test-stream-1";

  let mut stream = Stream::new(stream_name, 5, 10).unwrap();

  fn thread_function(stream_name: &str) -> Result<Vec<u64>, Error> {
    let mut stream = Stream::open(stream_name)?;
    let timeout = Duration::from_secs(5);
    let mut frames = Vec::new();
    while let Some(mut frame) = stream.pop(timeout)? {
      {
        let lock = frame.lock(timeout)?;
        frames.push(lock.frame_number());
      }
      stream.release_frame(frame).unwrap();
    }
    Ok(frames)
  }

  let frame_info = FrameInfo {
    frame_rate: Fraction {
      numerator: 25,
      denominator: 1,
    },
    video: Some(VideoInfo {
      resolution: Resolution {
        width: 1920,
        height: 1080,
        pixel_aspect_ratio: Fraction::one(),
      },
      pixel_format: PixelFormat::RGBA8,
      field_order: FieldOrder::Progressive,
    }),
    audio: Some(AudioInfo {
      sample_rate: 48000,
      channel_count: 2,
      format: crate::AudioFormat::S16,
    }),
  };

  let join_handle = thread::spawn(move || {
    thread_function(stream_name).unwrap()
  });

  let duration = Duration::from_secs(5);
  let count = 100;
  for frame_number in 0..count {
    let mut frame = stream.allocate(frame_info).unwrap();
    {
      let mut lock = frame.lock_mut(duration).unwrap();
      lock.set_frame_number(frame_number);
    }

    stream.push(&mut Some(frame), duration).unwrap();
  }

  stream.close().unwrap();

  let result = join_handle.join().unwrap();

  assert_eq!(result.len(), count as usize);
  for (expected, frame_number) in result.into_iter().enumerate() {
    assert_eq!(expected, frame_number as usize);
  }
}

#[test]
fn test_stream_2_pool_migration() {
  let stream_name = "bifrost-frames-test-stream-2";

  let mut stream = Stream::new(stream_name, 5, 10).unwrap();

  fn thread_function(stream_name: &str) -> Result<Vec<u64>, Error> {
    let mut stream = Stream::open(stream_name)?;
    let timeout = Duration::from_secs(5);
    let mut frames = Vec::new();

    let mut held_frames = Vec::new();
    while let Some(mut frame) = stream.pop(timeout)? {
      {
        let lock = frame.lock(timeout)?;
        frames.push(lock.frame_number());
      }

      held_frames.push(frame);
      if held_frames.len() > 20 {
        for frame in held_frames.drain(..) {
          stream.release_frame(frame)?;
        }
      }
    }
    for frame in held_frames.drain(..) {
      stream.release_frame(frame)?;
    }
    let stats = stream.stats()?;
    println!("Child thread: {stats:?}");
    Ok(frames)
  }

  fn function_main_loop(
    count: usize,
    stream: &mut Stream,
  ) -> Result<(), Error> {
    let frame_info = FrameInfo {
      frame_rate: Fraction {
        numerator: 25,
        denominator: 1,
      },
      video: Some(VideoInfo {
        resolution: Resolution {
          width: 1920,
          height: 1080,
          pixel_aspect_ratio: Fraction::one(),
        },
        pixel_format: PixelFormat::RGBA8,
        field_order: FieldOrder::Progressive,
      }),
      audio: Some(AudioInfo {
        sample_rate: 48000,
        channel_count: 2,
        format: crate::AudioFormat::S16,
      }),
    };

    let timeout = Duration::from_secs(5);
    for frame_number in 0..count {
      let mut frame = stream.allocate(frame_info)?;
      {
        let mut lock = frame.lock_mut(timeout)?;
        lock.set_frame_number(frame_number as u64);
      }

      stream.push(&mut Some(frame), timeout)?;
    }

    Ok(())
  }


  let join_handle = thread::spawn(move || {
    thread_function(stream_name)
  });

  let count = 1000;

  let main_result = function_main_loop(count, &mut stream);

  stream.close().unwrap();

  let thread_result = join_handle.join().unwrap();

  match main_result {
    Err(Error::Timeout) => {
      // If main thread timed out it is most likely due to the thread failed
      if let Err(thread_error) = thread_result {
        panic!("Child thread failed with error: {:?}", thread_error);
      } else {
        panic!("Main thread timed out!");
      }
    },
    Err(error) => {
      panic!("Main thread failed with error: {:?}", error);
    },
    Ok(()) => { }
  }

  let result = thread_result.unwrap();

  assert_eq!(result.len(), count as usize);
  for (expected, frame_number) in result.into_iter().enumerate() {
    assert_eq!(expected, frame_number as usize);
  }

  let stats = stream.stats().unwrap();

  assert!(stats.frames_allocated < 30);
  assert!(stats.queue_migrations < 3);
  // We should never map the same frame twice, so we should never map more
  // frames than have been allocated
  assert!(stats.frames_mapped <= stats.frames_allocated);

  println!("Main thread: {stats:?}");
}

#[test]
fn test_stream_3_return_queue_ownership() {
  let stream_name = "bifrost-frames-test-stream-3";

  let _owned_stream = Stream::new(stream_name, 1, 1).unwrap();

  let frame_info = FrameInfo {
    frame_rate: Fraction {
      numerator: 25,
      denominator: 1,
    },
    video: Some(VideoInfo {
      resolution: Resolution {
        width: 1920,
        height: 1080,
        pixel_aspect_ratio: Fraction::one(),
      },
      pixel_format: PixelFormat::RGBA8,
      field_order: FieldOrder::Progressive,
    }),
    audio: Some(AudioInfo {
      sample_rate: 48000,
      channel_count: 2,
      format: crate::AudioFormat::S16,
    }),
  };

  {
    let mut non_owner_stream = Stream::open(stream_name).unwrap();

    let mut frames = Vec::new();
    for _ in 0..5 {
      let frame = non_owner_stream.allocate(frame_info).unwrap();
      frames.push(frame);
    }
    for frame in frames.into_iter() {
      non_owner_stream.release_frame(frame).unwrap();
    }
  }

  {
    let _ = Stream::open(stream_name).unwrap();
  }
}
