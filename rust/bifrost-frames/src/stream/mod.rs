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
  return_queue_generation_id: u32,
  return_queue: FrameQueue,
  cache: FrameCache,
  stream: bifrost::Shm<StreamShm>,
}

#[derive(Copy, Clone, Debug)]
pub struct StreamStats {
  pub frames_mapped: usize,
  pub frames_allocated: usize,
  pub queue_migrations: usize,
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
      return_queue_generation_id,
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

    let (queue, return_queue, return_queue_generation_id) = {
      let lock = stream.data()?.mutable.to_lock(TIMEOUT)?;

      let queue_name = queue_shm_name(prefix)?;

      let return_queue_name =
        return_queue_shm_name(prefix, lock.return_queue_generation_id)?;

      let queue = FrameQueue::open(queue_name)?;
      let return_queue = FrameQueue::open(return_queue_name)?;
      (queue, return_queue, lock.return_queue_generation_id)
    };

    let cache = FrameCache::new();

    let stream = Stream {
      prefix,
      cache,
      queue,
      return_queue_generation_id,
      return_queue,
      stream,
    };

    Ok(stream)
  }

  pub fn stats(&mut self) -> Result<StreamStats, Error> {
    let lock = self.stream.data()?.mutable.to_lock(TIMEOUT)?;
    Ok(StreamStats {
      frames_allocated: lock.frame_id_counter as usize,
      frames_mapped: self.cache.stats().cache_misses,
      queue_migrations: lock.return_queue_generation_id as usize,
    })
  }

  pub fn allocate(&mut self, frame_info: FrameInfo) -> Result<Frame, Error> {
    loop {
      match self.return_queue.try_pop(Some(&mut self.cache)) {
        Ok(Some(mut frame)) => {
          if frame.info()? != frame_info {
            return Err(Error::StreamFormatChange);
          }
          return Ok(frame);
        },
        Ok(None) => {
          if self.try_update_return_queue()? {
            // If the return queue was migrated we switch to that now.
            continue;
          }
          let frame_id = {
            let mut lock = self.stream.data()?.mutable.to_lock_mut(TIMEOUT)?;
            let frame_id = lock.frame_id_counter;
            lock.frame_id_counter += 1;
            frame_id
          };

          let name = frame_shm_name(self.prefix, frame_id)?;
          let frame = Frame::new(name, 0, frame_info)?;
          return Ok(frame)
        },
        Err(error) => {
          return Err(error.into());
        }
      }
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

  /**
   *  Update the local instance of the return queue if a remote connection
   *  has migrated the return queue. Returns true if the queue was indeed
   *  migrated and the return queue has been updated. If the queue has not
   *  been migrated this function returns false.
   */
  fn try_update_return_queue(&mut self) -> Result<bool, Error> {
    let mut lock = self.stream.data()?.mutable.to_lock_mut(TIMEOUT)?;
    try_update_return_queue(
      &mut lock,
      &mut self.return_queue_generation_id,
      &mut self.return_queue,
      self.prefix
    )
  }

  /**
   *  Under a continuous lock, check first if another process has migrated the
   *  return queue and if not migrate the return queue to a queue of double
   *  size
   */
  fn double_return_queue_size_or_update_return_queue(&mut self) -> Result<(), Error> {
    let mut lock = self.stream.data()?.mutable.to_lock_mut(TIMEOUT)?;

    // Check first if another process has already migrated the pool.
    if try_update_return_queue(
      &mut lock,
      &mut self.return_queue_generation_id,
      &mut self.return_queue,
      self.prefix
    )? {
      return Ok(())
    }


    let return_queue_generation_id = lock.return_queue_generation_id + 1;

    let new_return_queue_name =
      return_queue_shm_name(
        self.prefix,
        return_queue_generation_id
      )?;

    let new_queue_size = self.return_queue.size() * 2;

    let new_queue =
      self.return_queue.migrate(
        new_return_queue_name,
        new_queue_size,
      )?;

    lock.return_queue_generation_id = return_queue_generation_id;
    self.return_queue_generation_id = return_queue_generation_id;
    self.return_queue = new_queue;
    Ok(())
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
          // The return queue is full. At this point we are not holding any
          // lock so we need to first check if someone has migrated the queue
          // in the meantime.
          self.double_return_queue_size_or_update_return_queue()?;
        },
        Err(Error::QueueClosed) => {
          // For multiple consumers it could be that another consumer has hit
          // the limit of the return queue and migrated it to a larger queue
          if !self.try_update_return_queue()? {
            // If the return queue is closed and not migrated it means that the
            // stream itself has been closed. This thread will clean up the
            // frame as there is no point in recycling frames anymore.
            return Ok(());
          }
        },
        Err(error) => {
          return Err(error.into());
        }
      }
    }
  }
}

fn try_update_return_queue<'l>(
  lock: &mut bifrost::OwnedLockMut<'l, bifrost::Platform, StreamShmMutable>,
  return_queue_generation_id: &mut u32,
  return_queue: &mut FrameQueue,
  prefix: bifrost::Name,
) -> Result<bool, Error> {
  if lock.return_queue_generation_id == *return_queue_generation_id {
    return Ok(false);
  }

  // Open the migrated queue
  let new_return_queue_name =
    return_queue_shm_name(
      prefix,
      lock.return_queue_generation_id
    )?;

  let new_queue = FrameQueue::open(new_return_queue_name)?;

  *return_queue_generation_id = lock.return_queue_generation_id;
  *return_queue = new_queue;
  return Ok(true);
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

