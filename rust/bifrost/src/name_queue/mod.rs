use std::time::{
  Duration,
  SystemTime,
};

use super::{
  derive::*,
  Error,
  Event,
  FourCC,
  MappableVec,
  Mutex,
  Name,
  PlatformTrait,
  Shm,
  ShmData,
};

mod interrupt_callback;
use interrupt_callback::InterruptCallback;

pub struct NameQueue<P: PlatformTrait> {
  size: usize,
  shm: Shm<QueueInternal<P>>,
  interrupt_callback: Option<InterruptCallback<P>>,
}

impl<P: PlatformTrait> NameQueue<P> {
  pub fn new(queue_name: Name, size: usize) -> Result<NameQueue<P>, Error> {
    let shm = Shm::new(queue_name, QueueInternal::new(size))?;
    Ok(NameQueue { size, shm, interrupt_callback: None })
  }

  pub fn open(queue_name: Name) -> Result<NameQueue<P>, Error> {
    let mut shm = Shm::<QueueInternal<P>>::open(queue_name)?;
    let size = {
      let lock = shm.data()?.mutable.to_lock(MUTEX_TIMEOUT)?;
      lock.elements.len()
    };
    Ok(NameQueue { size, shm, interrupt_callback: None })
  }

  pub fn register_interrupt_callback<F: FnMut() -> bool + Send + 'static>(
    &mut self,
    callback: F,
    interval: Duration,
  ) {
    let callback = InterruptCallback::new(self.shm.name(), callback, interval);
    self.interrupt_callback = Some(callback);
  }

  pub fn clear_interrupt_callback(&mut self) {
    self.interrupt_callback = None;
  }

  pub fn close(&mut self) -> Result<(), Error> {
    self.shm.data()?.close()
  }

  pub fn push(&mut self, name: Name, timeout: Duration) -> Result<(), Error> {
    self.shm.data()?.push(name, timeout, self.interrupt_callback.as_ref())
  }

  pub fn try_push(&mut self, name: Name) -> Result<bool, Error> {
    self.shm.data()?.try_push(name)
  }

  pub fn pop(&mut self, timeout: Duration) -> Result<Option<Name>, Error> {
    self.shm.data()?.pop(timeout, self.interrupt_callback.as_ref())
  }

  pub fn try_pop(&mut self) -> Result<Option<Name>, Error> {
    self.shm.data()?.try_pop()
  }

  pub fn size(&self) -> usize {
    self.size
  }

  fn interrupt(&mut self) -> Result<(), Error> {
    self.shm.data()?.interrupt()
  }

  pub fn migrate(
    &mut self,
    new_name: Name,
    new_size: usize
  ) -> Result<NameQueue<P>, Error> {
    let mut shm_data = self.shm.data()?;

    let mut lock = shm_data.mutable.to_lock_mut(MUTEX_TIMEOUT)?;

    let new_size = self.size.max(new_size);

    let mut new_queue = NameQueue::new(new_name, new_size)?;

    {
      let mut new_lock = new_queue.shm.data()?.mutable.to_lock_mut(MUTEX_TIMEOUT)?;

      while let Some(name) = lock.pop() {
        // No need to check for success here the new queue must be larger
        // because of the min expression above.
        new_lock.push(name);
      }

      lock.close();

      shm_data.name_available.signal()?;
      shm_data.slot_available.signal()?;
    }

    Ok(new_queue)
  }

  pub fn wait_until_drained(&mut self, timeout: Duration) -> Result<(), Error> {
    self.shm.data()?.wait_until_drained(timeout, self.interrupt_callback.as_ref())
  }

  pub fn is_owner(&self) -> bool {
    self.shm.is_owner()
  }

  pub fn release_ownership(&mut self) {
    self.shm.release_ownership();
  }

  pub fn acquire_ownership(&mut self) {
    self.shm.acquire_ownership();
  }
}


#[derive(MemoryMappable)]
struct QueueInternal<P: PlatformTrait> {
  name_available: Event<P>,
  slot_available: Event<P>,
  drained: Event<P>,
  mutable: Mutex<P, QueueMutable>,
}

impl<P: PlatformTrait> QueueInternal<P> {
  pub fn new(size: usize) -> QueueInternal<P> {
    QueueInternal {
      name_available: Event::new(),
      slot_available: Event::new(),
      drained: Event::new(),
      mutable: Mutex::new(QueueMutable::new(size)),
    }
  }
}

impl<P: PlatformTrait> ShmData for QueueInternal<P> {
  fn four_cc() -> FourCC {
    FourCC::from_static(b"nam1")
  }
}

struct TotalTimeout {
  start: SystemTime,
  timeout: Duration,
}

impl TotalTimeout {
  pub fn new(timeout: Duration) -> TotalTimeout {
    TotalTimeout {
      start: SystemTime::now(),
      timeout
    }
  }

  pub fn remain(&self) -> Result<Duration, Error> {
    let elapsed = self.start.elapsed()?;
    if elapsed >= self.timeout {
      return Err(Error::Timeout);
    }
    Ok(self.timeout - elapsed)
  }
}

impl<'t, P: PlatformTrait> QueueInternalRef<'t, P> {
  fn push(
    &mut self,
    name: Name, timeout: Duration,
    interrupt: Option<&InterruptCallback<P>>,
  ) -> Result<(), Error> {
    let timeout = TotalTimeout::new(timeout);
    loop {
      {
        let mut lock = self.mutable.lock_mut(MUTEX_TIMEOUT)?;
        if interrupt.map(|i| i.is_aborted()).unwrap_or(false) {
          return Err(Error::Interrupted);
        }
        if lock.is_closed() {
          return Err(Error::QueueClosed);
        }
        let was_empty = lock.empty();
        if lock.push(name) {
          if was_empty {
            self.name_available.signal()?;
          }
          return Ok(());
        }
        self.slot_available.clear()?;
      }
      self.slot_available.wait(
        timeout.remain()?,
        Some(&|| interrupt.as_ref().map(|i| i.is_aborted()).unwrap_or(false))
      )?;
    }
  }

  fn pop(
    &mut self,
    timeout: Duration,
    interrupt: Option<&InterruptCallback<P>>,
  ) -> Result<Option<Name>, Error> {
    let timeout = TotalTimeout::new(timeout);
    loop {
      {
        let mut lock = self.mutable.lock_mut(MUTEX_TIMEOUT)?;
        if interrupt.map(|i| i.is_aborted()).unwrap_or(false) {
          return Err(Error::Interrupted);
        }
        let was_full = lock.full();
        if let Some(name) = lock.pop() {
          if was_full {
            self.slot_available.signal()?;
          }
          if lock.empty() {
            self.drained.signal()?;
          }
          return Ok(Some(name));
        }

        if lock.is_closed() {
          return Ok(None)
        }

        self.name_available.clear()?;
      }
      self.name_available.wait(
        timeout.remain()?,
        Some(&|| interrupt.as_ref().map(|i| i.is_aborted()).unwrap_or(false))
      )?;
    }
  }

  fn try_pop(&mut self) -> Result<Option<Name>, Error> {
    let mut lock = self.mutable.lock_mut(MUTEX_TIMEOUT)?;
    let was_full = lock.full();
    if let Some(name) = lock.pop() {
      if was_full {
        self.slot_available.signal()?;
      }
      if lock.empty() {
        self.drained.signal()?;
      }
      return Ok(Some(name));
    }
    Ok(None)
  }

  fn interrupt(&mut self) -> Result<(), Error> {
    let mut _lock = self.mutable.lock_mut(MUTEX_TIMEOUT)?;
    self.name_available.signal()?;
    self.slot_available.signal()?;
    self.drained.signal()?;
    Ok(())
  }

  fn try_push(&mut self, name: Name) -> Result<bool, Error> {
    let mut lock = self.mutable.lock_mut(MUTEX_TIMEOUT)?;
    if lock.is_closed() {
      return Err(Error::QueueClosed);
    }
    let was_empty = lock.empty();
    if lock.push(name) {
      if was_empty {
        self.name_available.signal()?;
        self.drained.clear()?;
      }
      return Ok(true);
    } else {
      return Ok(false);
    }
  }

  fn wait_until_drained(
    &mut self,
    timeout: Duration,
    interrupt: Option<&InterruptCallback<P>>,
  ) -> Result<(), Error> {
    let timeout = TotalTimeout::new(timeout);
    loop {
      {
        let lock = self.mutable.lock_mut(MUTEX_TIMEOUT)?;
        if interrupt.map(|i| i.is_aborted()).unwrap_or(false) {
          return Err(Error::Interrupted);
        }
        if lock.empty() {
          return Ok(())
        }
        self.drained.clear()?;
      }
      self.drained.wait(
        timeout.remain()?,
        Some(&|| interrupt.as_ref().map(|i| i.is_aborted()).unwrap_or(false))
      )?;
    }
  }

  fn close(&mut self) -> Result<(), Error> {
    let mut lock = self.mutable.lock_mut(MUTEX_TIMEOUT)?;
    lock.close();
    self.name_available.signal()?;
    self.slot_available.signal()?;
    Ok(())
  }
}

// The mutex should never be held for long. If it is not acquired within
// 5 seconds something has gone wrong. The timeout on the mutex is not what is
// supposed to provide backpressure. That is the job of the signal which is
// why the timeout in the public API is only passed to the event wait. We use
// this hardcoded timeout on the mutex because if the mutex takes long time to
// acquire that would mean something has gone wrong and in that case all
// bets are off. Further more, we want to allow calling push/pop with zero
// timeout. If we passed zero timeout to the locking then this might timeout
// if another thread is holding the lock.
const MUTEX_TIMEOUT : Duration = Duration::from_secs(5);

#[derive(MemoryMappable)]
struct QueueMutable {
  closed: u32,
  read_head: u32,
  write_head: u32,
  elements: MappableVec<Name>,
}

impl QueueMutable {
  fn new(size: usize) -> QueueMutable {
    let empty_name : Name = Default::default();
    QueueMutable {
      closed: 0,
      read_head: 0,
      write_head: 0,
      elements: vec![empty_name; size].into(),
    }
  }
}

impl<'t> QueueMutableMut<'t> {
  fn is_closed(&self) -> bool {
    *self.closed == 1
  }

  fn close(&mut self) {
    *self.closed = 1;
  }

  fn empty(&self) -> bool {
    self.read_head == self.write_head
  }

  fn count(&self) -> usize {
    (*self.write_head - *self.read_head) as usize
  }

  fn full(&self) -> bool {
    self.count() == self.elements.len()
  }

  fn pop(&mut self) -> Option<Name> {
    if self.empty() {
      None
    } else {
      let index = *self.read_head as usize % self.elements.len();
      let name = self.elements[index];
      *self.read_head += 1;
      Some(name)
    }
  }

  fn push(&mut self, name: Name) -> bool {
    if self.full() {
      false
    } else {
      let index = *self.write_head as usize % self.elements.len();
      self.elements[index] = name;
      *self.write_head += 1;
      true
    }

  }
}

#[cfg(test)]
mod tests;
