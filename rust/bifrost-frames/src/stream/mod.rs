use std::time::Duration;

use bytemuck::{
  Pod,
  Zeroable,
};

use bifrost::derive::MemoryMappable;

use super::{
  Error,
  Frame,
  FrameCache,
  FrameInfo,
  FrameQueue,
};

pub struct Stream {
  prefix: bifrost::Name,
  queue: FrameQueue,
  return_queue: FrameQueue,
  cache: FrameCache,
  stream: bifrost::Shm<StreamShm>,
}


impl Stream {
  pub fn new(
    name: &str,
    queue_size: usize,
    pool_size: usize
  ) -> Result<Stream, Error> {
    let prefix = bifrost::Name::new(name)?;

    let return_queue_generation_id = 0;

    let queue_name = queue_shm_name(prefix)?;
    let return_queue_name = return_queue_shm_name(prefix, return_queue_generation_id)?;
    let stream_name = stream_bookkeeping_shm_name(prefix)?;

    let cache = FrameCache::new();
    let queue = FrameQueue::new(queue_name, queue_size)?;
    let return_queue = FrameQueue::new(return_queue_name, pool_size)?;

    let stream_shm = StreamShm {
      mutable: bifrost::Mutex::new(StreamShmMutable {
      return_queue_generation_id: return_queue_generation_id,
      frame_id_counter: 0,
      }),
    };

    let stream = bifrost::Shm::new(stream_name, stream_shm)?;

    let stream = Stream {
      prefix,
      cache,
      queue,
      return_queue,
      stream,
    };

    Ok(stream)
  }

  pub fn open(name: &str) -> Result<Stream, Error> {
    let prefix = bifrost::Name::new(name)?;
    let stream_name = stream_bookkeeping_shm_name(prefix)?;
    let mut stream = match bifrost::Shm::<StreamShm>::open(stream_name) {
      Ok(stream) => stream,
      Err(bifrost::Error::FourCCMismatch { .. }) => {
        return Err(Error::NotAStream);
      },
      Err(other) => {
        return Err(other.into());
      }
    };

    let (queue, return_queue) = {
      let lock = stream.data()?.mutable.to_lock(TIMEOUT)?;

      let queue_name = queue_shm_name(prefix)?;

      let return_queue_name =
        return_queue_shm_name(prefix, lock.return_queue_generation_id)?;

      let queue = FrameQueue::open(queue_name)?;
      let return_queue = FrameQueue::open(return_queue_name)?;
      (queue, return_queue)
    };

    let cache = FrameCache::new();

    let stream = Stream {
      prefix,
      cache,
      queue,
      return_queue,
      stream,
    };

    Ok(stream)
  }

  pub fn allocate(&mut self, frame_info: FrameInfo) -> Result<Frame, Error> {
    match self.return_queue.try_pop(Some(&mut self.cache)) {
      Ok(Some(mut frame)) => {
        if frame.info()? != frame_info {
          return Err(Error::StreamFormatChange);
        }
        Ok(frame)
      },
      Ok(None) => {
        let frame_id = {
          let mut lock = self.stream.data()?.mutable.to_lock_mut(TIMEOUT)?;
          let frame_id = lock.frame_id_counter;
          lock.frame_id_counter += 1;
          frame_id
        };

        let name = frame_shm_name(self.prefix, frame_id)?;
        let frame = Frame::new(name, 0, frame_info)?;
        Ok(frame)
      },
      Err(error) => Err(error.into()),
    }
  }

  pub fn close(&mut self) -> Result<(), Error> {
    self.queue.close().and(self.return_queue.close())
  }

  pub fn push(
    &mut self,
    frame: &mut Option<Frame>,
    timeout: Duration,
  ) -> Result<(), Error> {
    self.queue.push(frame, timeout, Some(&mut self.cache))
  }

  pub fn pop(
    &mut self,
    timeout: Duration,
  ) -> Result<Option<Frame>, Error> {
    self.queue.pop(timeout, Some(&mut self.cache))
  }

  pub fn release_frame(
    &mut self,
    frame: Frame,
  ) -> Result<(), Error> {
    let mut frame = Some(frame);

    loop {
      match self.return_queue.try_push(&mut frame, Some(&mut self.cache)) {
        Ok(true) => {
          return Ok(());
        },
        Ok(false) => {
          return Err(Error::ReturnQueueFull);
        },
        Err(Error::QueueClosed) => {
          // The stream is closed. This thread will clean up the frame
          return Ok(());
        },
        Err(error) => {
          return Err(error.into());
        }
      }
    }
  }
}

const TIMEOUT : Duration = Duration::from_secs(5);

fn stream_bookkeeping_shm_name(
  prefix: bifrost::Name
) -> Result<bifrost::Name, bifrost::Error> {
  bifrost::Name::new(&format!("{}-stream", prefix.to_str()?))
}

fn queue_shm_name(
  prefix: bifrost::Name
) -> Result<bifrost::Name, bifrost::Error> {
  bifrost::Name::new(&format!("{}-queue", prefix.to_str()?))
}

fn return_queue_shm_name(
  prefix: bifrost::Name,
  generation: u32,
) -> Result<bifrost::Name, bifrost::Error> {
  bifrost::Name::new(&format!("{}-return-queue-{}", prefix.to_str()?, generation))
}

fn frame_shm_name(
  prefix: bifrost::Name,
  frame_id: u32
) -> Result<bifrost::Name, bifrost::Error> {
  bifrost::Name::new(&format!("{}-frame-{}", prefix.to_str()?, frame_id))
}

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C)]
struct StreamShmMutable {
  return_queue_generation_id: u32,
  frame_id_counter: u32,
}

#[derive(MemoryMappable)]
struct StreamShm {
  mutable: bifrost::Mutex<bifrost::Platform, StreamShmMutable>,
}

impl bifrost::ShmData for StreamShm {
  fn four_cc() -> bifrost::FourCC {
    bifrost::FourCC::from_static(b"fst1")
  }
}

#[cfg(test)]
mod tests;

