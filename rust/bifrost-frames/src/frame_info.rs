use bytemuck::{
  Pod,
  Zeroable,
};

use super::Error;

#[derive(Pod, Zeroable, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Fraction {
  pub numerator: u32,
  pub denominator: u32
}

impl Fraction {
  pub fn one() -> Fraction {
    Fraction {
      numerator: 1,
      denominator: 1,
    }
  }
}

pub const MAX_RESOLUTION_DIMENSION : u32 = 32 * 1024;

#[derive(Pod, Zeroable, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Resolution {
  pub width: u32,
  pub height: u32,
  pub pixel_aspect_ratio: Fraction,
}

impl Resolution {
  pub fn is_valid(&self) -> bool {
    (self.width != 0) &&
    (self.height != 0) &&
    (self.width <= MAX_RESOLUTION_DIMENSION) &&
    (self.height <= MAX_RESOLUTION_DIMENSION) &&
    (self.pixel_aspect_ratio.denominator != 0) &&
    (self.pixel_aspect_ratio.numerator != 0)
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PixelFormat {
  RGBA8,
  RGBA16,
}

impl PixelFormat {
  pub fn from_u32(value: u32) -> Result<PixelFormat, Error> {
    match value {
      0 => Ok(PixelFormat::RGBA8),
      1 => Ok(PixelFormat::RGBA16),
      _ => Err(Error::BadEnumValue),
    }
  }

  pub fn to_u32(self) -> u32 {
    match self {
      PixelFormat::RGBA8 => 0,
      PixelFormat::RGBA16 => 1,
    }
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FieldOrder {
  Progressive,
  TopFieldFirst,
  BottomFieldFirst,
}

impl FieldOrder {
  pub fn from_u32(value: u32) -> Result<FieldOrder, Error> {
    match value {
      0 => Ok(FieldOrder::Progressive),
      1 => Ok(FieldOrder::TopFieldFirst),
      2 => Ok(FieldOrder::BottomFieldFirst),
      _ => Err(Error::BadEnumValue),
    }
  }

  pub fn to_u32(self) -> u32 {
    match self {
      FieldOrder::Progressive => 0,
      FieldOrder::TopFieldFirst => 1,
      FieldOrder::BottomFieldFirst => 2,
    }
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct VideoInfo {
  pub resolution: Resolution,
  pub pixel_format: PixelFormat,
  pub field_order: FieldOrder,
}

impl VideoInfo {
  pub fn buffer_size(&self) -> usize {
    let pixel_count =
      self.resolution.width as usize * self.resolution.height as usize;

    match self.pixel_format {
      PixelFormat::RGBA8 => pixel_count * 4,
      PixelFormat::RGBA16 => pixel_count * 8,
    }
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AudioFormat {
  S16,
  F32,
}

impl AudioFormat {
  pub fn from_u32(value: u32) -> Result<AudioFormat, Error> {
    match value {
      0 => Ok(AudioFormat::S16),
      1 => Ok(AudioFormat::F32),
      _ => Err(Error::BadEnumValue),
    }
  }

  pub fn to_u32(self) -> u32 {
    match self {
      AudioFormat::S16 => 0,
      AudioFormat::F32 => 1,
    }
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct AudioInfo {
  pub sample_rate: u32,
  pub channel_count: u32,
  pub format: AudioFormat,
}

impl AudioInfo {
  pub fn sample_offset(&self, frame_rate: Fraction, frame_number: u64) -> u64 {
    ((self.sample_rate as u128 * frame_number as u128 * frame_rate.denominator as u128)
      / frame_rate.numerator as u128) as u64
  }

  pub fn sample_count(&self, frame_rate: Fraction, frame_number: u64) -> u64 {
    self.sample_offset(frame_rate, frame_number + 1)
      - self.sample_offset(frame_rate, frame_number)
  }

  pub fn max_sample_count(&self, frame_rate: Fraction) -> u64 {
    (((self.sample_rate as u128 * frame_rate.denominator as u128) + (frame_rate.numerator as u128 - 1))
      / frame_rate.numerator as u128) as u64
  }

  pub fn buffer_size(&self, sample_count: u64) -> usize {
    let total_sample_count =
      sample_count as usize * self.channel_count as usize;

    match self.format {
      AudioFormat::S16 => total_sample_count * 2,
      AudioFormat::F32 => total_sample_count * 4,
    }
  }

  pub fn max_buffer_size(&self, frame_rate: Fraction) -> usize {
    let sample_count = self.max_sample_count(frame_rate);
    self.buffer_size(sample_count)
  }

}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FrameInfo {
  pub frame_rate: Fraction,
  pub video: Option<VideoInfo>,
  pub audio: Option<AudioInfo>,
}


