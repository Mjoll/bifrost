use std::time::Duration;

use bifrost::{
  OwnedLock,
  OwnedLockMut,
  Name,
  Platform,
  Shm,
};

use super::{
  AudioFormat,
  AudioInfo,
  Error,
  FieldOrder,
  Fraction,
  FrameInfo,
  PixelFormat,
  Resolution,
  ShmFrame,
  ShmFrameHeader,
  ShmFrameMutable,
  VideoInfo,
};

pub struct Frame {
  shm: Shm<ShmFrame>
}

impl Frame {
  pub fn new(
    name: Name,
    frame_number: u64,
    info: FrameInfo,
  ) -> Result<Frame, Error> {
    let shm_frame = ShmFrame::new(frame_number, info)?;
    let shm = Shm::new(name, shm_frame)
      .map_err(|e| {
        match e {
          bifrost::Error::AlreadyExists => Error::AlreadyExists,
          other => other.into(),
        }
      })?;

    Ok(Frame { shm })
  }

  pub fn open(name: Name) -> Result<Frame, Error> {
    let shm = Shm::open(name)
      .map_err(|e| {
        match e {
          bifrost::Error::NotFound => Error::NotFound,
          bifrost::Error::FourCCMismatch { .. } => Error::NotAFrame,
          other => other.into(),
        }
      })?;
    Ok(Frame { shm })
  }

  pub fn info(&mut self) -> Result<FrameInfo, Error> {
    let shm_frame = self.shm.data()?;


    let frame_info = FrameInfo {
      frame_rate: shm_frame.header.frame_rate,
      video: shm_frame.header.video_info.try_into()?,
      audio: shm_frame.header.audio_info.try_into()?,
    };

    Ok(frame_info)
  }

  pub fn lock(&mut self, timeout: Duration) -> Result<FrameLock<'_>, Error> {
    let shm_frame = self.shm.data()?;
    Ok(FrameLock {
      lock: shm_frame.mutable.to_lock(timeout)?,
      header: shm_frame.header,
    })
  }

  pub fn lock_mut(&mut self, timeout: Duration) -> Result<FrameLockMut<'_>, Error> {
    let shm_frame = self.shm.data()?;
    Ok(FrameLockMut {
      lock: shm_frame.mutable.to_lock_mut(timeout)?,
      header: shm_frame.header,
    })
  }
}

pub struct VideoFrameRef<'t> {
  pub resolution: Resolution,
  pub pixel_format: PixelFormat,
  pub field_order: FieldOrder,
  pub buffer: &'t [u8],
}

pub struct AudioFrameRef<'t> {
  pub sample_rate: u32,
  pub sample_offset: u64,
  pub sample_count: u64,
  pub channel_count: u32,
  pub format: AudioFormat,
  pub buffer: &'t [u8],
}

pub struct FrameLock<'t> {
  lock: OwnedLock<'t, Platform, ShmFrameMutable>,
  header: &'t ShmFrameHeader,
}

impl<'t> FrameLock<'t> {
  pub fn frame_number(&self) -> u64 {
    *self.lock.frame_number
  }

  pub fn frame_rate(&self) -> Fraction {
    self.header.frame_rate
  }

  pub fn video(&self) -> Result<Option<VideoFrameRef<'_>>, Error> {
    let video_info : Option<VideoInfo> = self.header.video_info.try_into()?;
    match video_info {
      Some(video_info) => {
        let buffer_size = video_info.buffer_size();

        let video_frame = VideoFrameRef {
          resolution: video_info.resolution,
          pixel_format: video_info.pixel_format,
          field_order: video_info.field_order,
          buffer: &self.lock.buffer[0..buffer_size],
        };

        Ok(Some(video_frame))
      },
      None => Ok(None)
    }
  }

  pub fn audio(&self) -> Result<Option<AudioFrameRef<'_>>, Error> {
    let audio_info : Option<AudioInfo> = self.header.audio_info.try_into()?;
    match audio_info {
      Some(audio_info) => {
        let video_info : Option<VideoInfo> =
          self.header.video_info.try_into()?;

        let video_buffer_size = video_info
          .map(|info| info.buffer_size())
          .unwrap_or(0);

        let frame_rate = self.header.frame_rate;
        let frame_number = *self.lock.frame_number;

        let sample_count = audio_info.sample_count(frame_rate, frame_number);
        let sample_offset = audio_info.sample_offset(frame_rate, frame_number);

        let audio_buffer_size = audio_info.buffer_size(sample_count);

        let start = video_buffer_size;
        let end = start + audio_buffer_size;

        let audio_frame = AudioFrameRef {
          sample_rate: audio_info.sample_rate,
          sample_offset,
          sample_count,
          channel_count: audio_info.channel_count,
          format: audio_info.format,
          buffer: &self.lock.buffer[start..end],
        };

        Ok(Some(audio_frame))
      },
      None => {
        Ok(None)
      }
    }
  }
}

pub struct VideoFrameMut<'t> {
  pub resolution: Resolution,
  pub pixel_format: PixelFormat,
  pub field_order: FieldOrder,
  pub buffer: &'t mut [u8],
}

pub struct AudioFrameMut<'t> {
  pub sample_rate: u32,
  pub sample_offset: u64,
  pub sample_count: u64,
  pub channel_count: u32,
  pub format: AudioFormat,
  pub buffer: &'t mut [u8],
}

pub struct FrameLockMut<'t> {
  lock: OwnedLockMut<'t, Platform, ShmFrameMutable>,
  header: &'t ShmFrameHeader,
}

impl<'t> FrameLockMut<'t> {
  pub fn frame_number(&mut self) -> u64 {
    *self.lock.frame_number
  }

  pub fn set_frame_number(&mut self, frame_number: u64) {
    *self.lock.frame_number = frame_number;
  }

  pub fn frame_rate(&self) -> Fraction {
    self.header.frame_rate
  }

  pub fn video(&mut self) -> Result<Option<VideoFrameMut<'_>>, Error> {
    let video_info : Option<VideoInfo> = self.header.video_info.try_into()?;
    match video_info {
      Some(video_info) => {
        let buffer_size = video_info.buffer_size();

        let video_frame = VideoFrameMut {
          resolution: video_info.resolution,
          pixel_format: video_info.pixel_format,
          field_order: video_info.field_order,
          buffer: &mut self.lock.buffer[0..buffer_size],
        };

        Ok(Some(video_frame))
      },
      None => {
        Ok(None)
      },
    }
  }

  pub fn audio(&mut self) -> Result<Option<AudioFrameMut<'_>>, Error> {
    let audio_info : Option<AudioInfo> = self.header.audio_info.try_into()?;
    match audio_info {
      Some(audio_info) => {
        let video_info : Option<VideoInfo> =
          self.header.video_info.try_into()?;

        let video_buffer_size = video_info
          .map(|info| info.buffer_size())
          .unwrap_or(0);

        let frame_rate = self.header.frame_rate;
        let frame_number = *self.lock.frame_number;

        let sample_count = audio_info.sample_count(frame_rate, frame_number);
        let sample_offset = audio_info.sample_offset(frame_rate, frame_number);

        let audio_buffer_size = audio_info.buffer_size(sample_count);

        let start = video_buffer_size;
        let end = start + audio_buffer_size;

        let audio_frame = AudioFrameMut {
          sample_rate: audio_info.sample_rate,
          sample_offset,
          sample_count,
          channel_count: audio_info.channel_count,
          format: audio_info.format,
          buffer: &mut self.lock.buffer[start..end],
        };

        Ok(Some(audio_frame))
      },
      None => {
        Ok(None)
      }
    }
  }
}

