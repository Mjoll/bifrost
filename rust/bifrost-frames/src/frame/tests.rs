use std::time::Duration;

use bytemuck::{
  Pod,
  Zeroable,
};

use bifrost::Name;

use super::{
  AudioFormat,
  AudioInfo,
  Error,
  FieldOrder,
  Fraction,
  Frame,
  FrameInfo,
  PixelFormat,
  Resolution,
  VideoInfo,
};

#[test]
fn simple_frame_test_1() {
  let name = Name::new("bifrost-frames-test-1-frame-0").unwrap();
  bifrost::delete_shm(name).unwrap();

  let frame_rate = Fraction {
    numerator: 25,
    denominator: 1,
  };

  let resolution = Resolution {
    width: 1920,
    height: 1080,
    pixel_aspect_ratio: Fraction::one(),
  };

  let frame_info = FrameInfo {
    frame_rate,
    video: Some(VideoInfo {
      resolution,
      pixel_format: PixelFormat::RGBA16,
      field_order: FieldOrder::Progressive,
    }),
    audio: Some(AudioInfo {
      sample_rate: 48000,
      channel_count: 2,
      format: AudioFormat::S16,
    })
  };

  let timeout = Duration::from_secs(5);

  let mut frame = Frame::new(name, 1, frame_info).unwrap();

  {
    let mut lock = frame.lock_mut(timeout).unwrap();

    assert_eq!(lock.frame_number(), 1);
    assert_eq!(lock.frame_rate(), frame_rate);
    lock.set_frame_number(2);

    let video = lock.video().unwrap().unwrap();
    assert_eq!(video.resolution, resolution);
    assert_eq!(video.pixel_format, PixelFormat::RGBA16);
    assert_eq!(video.field_order, FieldOrder::Progressive);
    assert_eq!(video.buffer.len(), 1920 * 1080 * 8);
    video.buffer.fill(65);

    let audio = lock.audio().unwrap().unwrap();
    assert_eq!(audio.sample_rate, 48000);
    assert_eq!(audio.channel_count, 2);
    assert_eq!(audio.format, AudioFormat::S16);
    assert_eq!(audio.sample_count, 48000 / 25);
    assert_eq!(audio.sample_offset, 48000 * 2 / 25);
    assert_eq!(audio.buffer.len(), 48000 * 2 * 2 / 25);
    audio.buffer.fill(64);
  }

  let mut frame2 = Frame::open(name).unwrap();

  {
    let lock = frame2.lock(timeout).unwrap();

    assert_eq!(lock.frame_number(), 2);
    assert_eq!(lock.frame_rate(), frame_rate);

    let video = lock.video().unwrap().unwrap();
    assert_eq!(video.resolution, resolution);
    assert_eq!(video.pixel_format, PixelFormat::RGBA16);
    assert_eq!(video.field_order, FieldOrder::Progressive);
    assert_eq!(video.buffer.len(), 1920 * 1080 * 8);
    assert_eq!(video.buffer.iter().filter(|v| **v != 65).count(), 0);

    let audio = lock.audio().unwrap().unwrap();
    assert_eq!(audio.sample_rate, 48000);
    assert_eq!(audio.channel_count, 2);
    assert_eq!(audio.format, AudioFormat::S16);
    assert_eq!(audio.buffer.len(), 48000 * 2 * 2 / 25);
    assert_eq!(audio.buffer.iter().filter(|v| **v != 64).count(), 0);
  }
}

#[test]
fn simple_frame_test_2() {
  let name = Name::new("bifrost-frames-test-2-frame-0").unwrap();
  bifrost::delete_shm(name).unwrap();

  let frame_rate = Fraction {
    numerator: 30000,
    denominator: 1001,
  };

  let resolution = Resolution {
    width: 1280,
    height: 720,
    pixel_aspect_ratio: Fraction::one(),
  };

  let frame_info = FrameInfo {
    frame_rate,
    video: Some(VideoInfo {
      resolution,
      pixel_format: PixelFormat::RGBA8,
      field_order: FieldOrder::TopFieldFirst,
    }),
    audio: Some(AudioInfo {
      sample_rate: 44100,
      channel_count: 8,
      format: AudioFormat::F32,
    })
  };

  let timeout = Duration::from_secs(5);

  let mut frame = Frame::new(name, 1, frame_info).unwrap();

  {
    let mut lock = frame.lock_mut(timeout).unwrap();

    assert_eq!(lock.frame_number(), 1);
    assert_eq!(lock.frame_rate(), frame_rate);
    lock.set_frame_number(2);

    let video = lock.video().unwrap().unwrap();
    assert_eq!(video.resolution, resolution);
    assert_eq!(video.pixel_format, PixelFormat::RGBA8);
    assert_eq!(video.field_order, FieldOrder::TopFieldFirst);
    assert_eq!(video.buffer.len(), 1280 * 720 * 4);
    video.buffer.fill(65);

    let audio = lock.audio().unwrap().unwrap();
    assert_eq!(audio.sample_rate, 44100);
    assert_eq!(audio.channel_count, 8);
    assert_eq!(audio.format, AudioFormat::F32);
    assert_eq!(audio.sample_count, 1472);
    // 44100 / (30000 / 1001) = 1741.47
    assert_eq!(audio.buffer.len(), 1472 * 4 * 8);
    audio.buffer.fill(64);
  }

  let mut frame2 = Frame::open(name).unwrap();

  {
    let lock = frame2.lock(timeout).unwrap();

    assert_eq!(lock.frame_number(), 2);
    assert_eq!(lock.frame_rate(), frame_rate);

    let video = lock.video().unwrap().unwrap();
    assert_eq!(video.resolution, resolution);
    assert_eq!(video.pixel_format, PixelFormat::RGBA8);
    assert_eq!(video.field_order, FieldOrder::TopFieldFirst);
    assert_eq!(video.buffer.len(), 1280 * 720 * 4);
    assert_eq!(video.buffer.iter().filter(|v| **v != 65).count(), 0);

    let audio = lock.audio().unwrap().unwrap();
    assert_eq!(audio.sample_rate, 44100);
    assert_eq!(audio.channel_count, 8);
    assert_eq!(audio.format, AudioFormat::F32);
    assert_eq!(audio.sample_count, 1472);
    assert_eq!(audio.buffer.len(), 1472 * 4 * 8);
    assert_eq!(audio.buffer.iter().filter(|v| **v != 64).count(), 0);
  }

  // Check that mutating the frame number changes the number of samples in the
  // audio frame
  {
    let mut offset = 0;
    let mut lock = frame2.lock_mut(timeout).unwrap();
    {
      lock.set_frame_number(0);
      let audio = lock.audio().unwrap().unwrap();
      assert_eq!(audio.sample_count, 1471);
      assert_eq!(audio.sample_offset, offset);
      assert_eq!(audio.buffer.len(), audio.sample_count as usize * 4 * 8);
      offset += audio.sample_count;
    }
    {
      lock.set_frame_number(1);
      let audio = lock.audio().unwrap().unwrap();
      assert_eq!(audio.sample_count, 1471);
      assert_eq!(audio.sample_offset, offset);
      assert_eq!(audio.buffer.len(), audio.sample_count as usize * 4 * 8);
      offset += audio.sample_count;
    }
    {
      lock.set_frame_number(2);
      let audio = lock.audio().unwrap().unwrap();
      assert_eq!(audio.sample_count, 1472);
      assert_eq!(audio.sample_offset, offset);
      assert_eq!(audio.buffer.len(), audio.sample_count as usize * 4 * 8);
      offset += audio.sample_count;
    }
    {
      lock.set_frame_number(3);
      let audio = lock.audio().unwrap().unwrap();
      assert_eq!(audio.sample_count, 1471);
      assert_eq!(audio.sample_offset, offset);
      assert_eq!(audio.buffer.len(), audio.sample_count as usize * 4 * 8);
    }
  }
}

#[test]
fn simple_frame_test_3() {
  let name = Name::new("bifrost-frames-test-3-frame-0").unwrap();
  bifrost::delete_shm(name).unwrap();

  match Frame::open(name) {
    Ok(_) => {
      panic!("Expected open to fail");
    },
    Err(Error::NotFound) => { },
    Err(other) => {
      panic!("Expected open to failed with NotFound but {other:?} was returned");
    }
  }

  let frame_rate = Fraction {
    numerator: 25,
    denominator: 1,
  };

  let frame_info = FrameInfo {
    frame_rate,
    video: None,
    audio: None,
  };

  let mut frame = Frame::new(name, 0, frame_info).unwrap();

  {
    let timeout = Duration::from_secs(5);
    let lock = frame.lock(timeout).unwrap();

    assert!(lock.video().unwrap().is_none());
    assert!(lock.audio().unwrap().is_none());
  }
  match Frame::new(name, 0, frame_info) {
    Ok(_) => {
      panic!("Expected Frame::new to fail");
    },
    Err(Error::AlreadyExists) => { },
    Err(other) => {
      panic!("Expected Frame::new to fail with AlreadyExists, but {other:?} was returned");
    }
  }


  let garbage_name = Name::new("bifrost-frames-test-3-garbage").unwrap();
  bifrost::delete_shm(garbage_name).unwrap();

  #[derive(Pod, Zeroable, Copy, Clone)]
  #[repr(C)]
  struct Garbage {
    value: u32
  }

  impl bifrost::ShmData for Garbage {
    fn four_cc() -> bifrost::FourCC {
      bifrost::FourCC::from_static(b"grbg")
    }
  }

  let init_garbage = Garbage {
    value: 5,
  };

  let garbage : bifrost::Shm<Garbage> =
    bifrost::Shm::new(garbage_name, init_garbage).unwrap();

  match Frame::open(garbage_name) {
    Ok(_) => {
      panic!("Expected Frame::open to fail");
    },
    Err(Error::NotAFrame) => { },
    Err(other) => {
      panic!("Expected Frame::open to fail with NotAFrame, but {other:?} was returned");
    },
  }

  drop(garbage);
}
