use bytemuck::{
  Pod,
  Zeroable,
};

use bifrost::{
  derive::MemoryMappable,
  FourCC,
  MappableVec,
  Mutex,
  Platform,
  ShmData,
};

use super::{
  AudioInfo,
  AudioFormat,
  Error,
  FieldOrder,
  Fraction,
  FrameInfo,
  PixelFormat,
  Resolution,
  VideoInfo,
};

#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct ShmVideoInfo {
  pub resolution: Resolution,
  pub pixel_format: u32,
  pub field_order: u32,
}

impl TryInto<Option<VideoInfo>> for ShmVideoInfo {
  type Error = Error;
  fn try_into(self) -> Result<Option<VideoInfo>, Error> {
    if self.resolution.width > 0 {
      Ok(Some(VideoInfo {
        resolution: self.resolution,
        pixel_format: PixelFormat::from_u32(self.pixel_format)?,
        field_order: FieldOrder::from_u32(self.field_order)?,
      }))
    } else {
      Ok(None)
    }
  }
}

impl TryFrom<Option<VideoInfo>> for ShmVideoInfo {
  type Error = Error;
  fn try_from(value: Option<VideoInfo>) -> Result<Self, Error> {
    Ok(match value {
      Some(info) => {
        if !info.resolution.is_valid() {
          return Err(Error::BadResolution(info.resolution));
        }
        ShmVideoInfo {
          resolution: info.resolution,
          pixel_format: info.pixel_format.to_u32(),
          field_order: info.field_order.to_u32(),
        }
      },
      None => {
        ShmVideoInfo {
          resolution: Resolution {
            width: 0,
            height: 0,
            pixel_aspect_ratio: Fraction::one(),
          },
          pixel_format: 0,
          field_order: 0,
        }
      }
    })
  }
}

#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct ShmAudioInfo {
  pub sample_rate: u32,
  pub channel_count: u32,
  pub format: u32,
}

impl TryInto<Option<AudioInfo>> for ShmAudioInfo {
  type Error = Error;
  fn try_into(self) -> Result<Option<AudioInfo>, Error> {
    if self.sample_rate != 0 {
      Ok(Some(AudioInfo {
        sample_rate: self.sample_rate,
        channel_count: self.channel_count,
        format: AudioFormat::from_u32(self.format)?,
      }))
    } else {
      Ok(None)
    }
  }
}

const MAX_SAMPLE_RATE : u32 = 192000;

impl TryFrom<Option<AudioInfo>> for ShmAudioInfo {
  type Error = Error;
  fn try_from(value: Option<AudioInfo>) -> Result<Self, Error> {
    Ok(match value {
      Some(info) => {
        if info.sample_rate == 0 || info.sample_rate > MAX_SAMPLE_RATE {
          return Err(Error::BadSampleRate(info.sample_rate));
        }
        ShmAudioInfo {
          sample_rate: info.sample_rate,
          channel_count: info.channel_count,
          format: info.format.to_u32(),
        }
      },
      None => {
        ShmAudioInfo {
          sample_rate: 0,
          channel_count: 0,
          format: 0,
        }
      }
    })
  }
}

#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct ShmFrameHeader {
  pub frame_rate: Fraction,
  pub video_info: ShmVideoInfo,
  pub audio_info: ShmAudioInfo,
}

#[derive(MemoryMappable)]
pub struct ShmFrameMutable {
  pub frame_number: u64,
  /**
   *  Buffer containing first the video followed by the audio
   */
  pub buffer: MappableVec<u8>,
}

#[derive(MemoryMappable)]
pub struct ShmFrame {
  header: ShmFrameHeader,
  mutable: Mutex<Platform, ShmFrameMutable>,
}

impl ShmFrame {
  pub fn new(frame_number: u64, info: FrameInfo) -> Result<ShmFrame, Error> {
    if info.frame_rate.denominator == 0 || info.frame_rate.numerator == 0 {
      return Err(Error::BadFrameRate(info.frame_rate));
    }
    let frame_rate = info.frame_rate;

    let header = ShmFrameHeader {
      frame_rate,
      video_info: info.video.try_into()?,
      audio_info: info.audio.try_into()?,
    };

    let video_buffer_size =
      match info.video {
        Some(info) => info.buffer_size(),
        None => 0,
      };

    let audio_buffer_size =
      match info.audio {
        Some(info) => {
          let max_sample_count = info.max_sample_count(frame_rate);
          info.buffer_size(max_sample_count)
        },
        None => { 0 }
      };

    let buffer_size = video_buffer_size + audio_buffer_size;

    let mutable = ShmFrameMutable {
      frame_number,
      buffer: vec![0u8; buffer_size].into(),
    };

    Ok(ShmFrame { header, mutable: Mutex::new(mutable) })
  }
}

impl ShmData for ShmFrame {
  fn four_cc() -> FourCC {
    FourCC::from_static(b"frm1")
  }
}


