use std::time::Duration;

use bifrost::{
  Name,
  Platform,
};

use super::{
  Error,
  Frame,
  ShmFrame,
};

pub use bifrost::CacheStats as FrameCacheStats;

pub struct FrameCache {
  cache: bifrost::LocalCache<ShmFrame>,
}

impl FrameCache {
  pub fn new() -> FrameCache {
    FrameCache { cache: bifrost::LocalCache::new() }
  }

  pub fn stats(&self) -> FrameCacheStats {
    self.cache.stats()
  }
}

pub struct FrameQueue {
  queue: bifrost::Queue<Platform, ShmFrame>,
}

impl FrameQueue {
  pub fn new(queue_name: Name, size: usize) -> Result<FrameQueue, Error> {
    match bifrost::Queue::new(queue_name, size) {
      Ok(queue) => Ok(FrameQueue { queue }),
      Err(bifrost::Error::FourCCMismatch { .. }) => {
        Err(Error::NotAFrameQueue)
      },
      Err(bifrost::Error::AlreadyExists) => {
        Err(Error::AlreadyExists)
      },
      Err(other) => Err(other.into())
    }
  }

  pub fn open(queue_name: Name) -> Result<FrameQueue, Error> {
    match bifrost::Queue::open(queue_name) {
      Ok(queue) => Ok(FrameQueue { queue }),
      Err(bifrost::Error::FourCCMismatch { .. }) => {
        Err(Error::NotAFrameQueue)
      },
      Err(bifrost::Error::NotFound) => {
        Err(Error::NotFound)
      },
      Err(other) => Err(other.into())
    }
  }

  pub fn push(
    &mut self,
    in_frame: &mut Option<Frame>,
    timeout: Duration,
    cache: Option<&mut FrameCache>
  ) -> Result<(), Error> {
    let mut shm = in_frame.take().map(|frame| frame.unwrap());
    match self.queue.push(
      &mut shm,
      timeout,
      cache.map(|cache| &mut cache.cache)
    ) {
      Ok(()) => {
        Ok(())
      },
      Err(error) => {
        if let Some(shm) = shm {
          *in_frame = Some(Frame::wrap(shm));
        }
        Err(error.into())
      }
    }
  }

  pub fn pop(
    &mut self,
    timeout: Duration,
    cache: Option<&mut FrameCache>
  ) -> Result<Option<Frame>, Error> {
    Ok(self.queue.pop(
      timeout,
      cache.map(|cache| &mut cache.cache)
    ).map(|maybe_shm| maybe_shm.map(|shm| Frame::wrap(shm)))?)
  }

  pub fn try_pop(
    &mut self,
    cache: Option<&mut FrameCache>
  ) -> Result<Option<Frame>, Error> {
    Ok(self.queue.try_pop(
      cache.map(|cache| &mut cache.cache)
    ).map(|maybe_shm| maybe_shm.map(|shm| Frame::wrap(shm)))?)
  }

  pub fn try_push(
    &mut self,
    in_frame: &mut Option<Frame>,
    cache: Option<&mut FrameCache>
  ) -> Result<bool, Error> {
    let mut shm = in_frame.take().map(|frame| frame.unwrap());
    match self.queue.try_push(
      &mut shm,
      cache.map(|cache| &mut cache.cache)
    ) {
      Ok(true) => {
        Ok(true)
      },
      Ok(false) => {
        if let Some(shm) = shm {
          *in_frame = Some(Frame::wrap(shm))
        }
        Ok(false)
      },
      Err(error) => {
        if let Some(shm) = shm {
          *in_frame = Some(Frame::wrap(shm));
        }
        Err(error.into())
      }
    }
  }

  pub fn close(&mut self) -> Result<(), Error> {
    Ok(self.queue.close()?)
  }

  pub fn size(&self) -> usize {
    self.queue.size()
  }

  pub fn migrate(
    &mut self,
    new_name: Name,
    new_size: usize
  ) -> Result<FrameQueue, Error> {
    let queue = self.queue.migrate(new_name, new_size)?;
    Ok(FrameQueue { queue })
  }
}

