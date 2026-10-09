use std::{
  marker::PhantomData,
  time::Duration,
};

use super::{
  delete_shm,
  Error,
  Name,
  NameQueue,
  PlatformTrait,
  Shm,
  ShmData,
};

pub struct Queue<P: PlatformTrait, T: ShmData> {
  name_queue: NameQueue<P>,
  owner: bool,
  _marker: PhantomData<T>,
}

impl<P: PlatformTrait, T: ShmData> Queue<P, T> {
  pub fn new(queue_name: Name, size: usize) -> Result<Queue<P, T>, Error> {
    let queue = Queue {
      name_queue: NameQueue::new(queue_name, size)?,
      owner: true,
      _marker: Default::default(),
    };

    Ok(queue)
  }

  pub fn open(queue_name: Name) -> Result<Queue<P, T>, Error> {
    let queue = Queue {
      name_queue: NameQueue::open(queue_name)?,
      owner: false,
      _marker: Default::default(),
    };

    Ok(queue)
  }

  pub fn push(
    &mut self,
    in_shm: &mut Option<Shm<T>>,
    timeout: Duration,
  ) -> Result<(), Error> {
    let Some(mut shm) = in_shm.take() else {
      return Ok(())
    };

    match self.name_queue.push(shm.name(), timeout) {
      Ok(()) => { },
      Err(err) => {
        *in_shm = Some(shm);
        return Err(err);
      }
    }

    // The Shm no longer owns the memory, the queue does
    shm.release_ownership();

    Ok(())
  }

  pub fn try_push(
    &mut self,
    in_shm: &mut Option<Shm<T>>,
  ) -> Result<bool, Error> {
    let Some(mut shm) = in_shm.take() else {
      return Ok(false)
    };

    match self.name_queue.try_push(shm.name()) {
      Ok(false) => {
        *in_shm = Some(shm);
        return Ok(false);
      },
      Ok(true) => {

      },
      Err(err) => {
        *in_shm = Some(shm);
        return Err(err);
      }
    }

    // The Shm no longer owns the memory, the queue does
    shm.release_ownership();

    Ok(true)
  }

  pub fn pop(
    &mut self,
    timeout: Duration,
  ) -> Result<Option<Shm<T>>, Error> {
    let Some(name) = self.name_queue.pop(timeout)? else {
      return Ok(None);
    };


    let mut shm = Shm::open(name)?;
    shm.acquire_ownership();
    Ok(Some(shm))
  }

  pub fn try_pop(&mut self) -> Result<Option<Shm<T>>, Error> {
    let Some(name) = self.name_queue.try_pop()? else {
      return Ok(None);
    };


    let mut shm = Shm::open(name)?;
    shm.acquire_ownership();
    Ok(Some(shm))
  }

  pub fn close(&mut self) -> Result<(), Error> {
    self.name_queue.close()
  }
}

impl<P: PlatformTrait, T: ShmData> Drop for Queue<P, T> {
  fn drop(&mut self) {
    if !self.owner {
      return;
    }
    if self.name_queue.close().is_err() {
      // Something went wrong. Nothing we can do here
      return;
    }
    while let Ok(Some(name)) = self.name_queue.try_pop() {
      let _ = delete_shm(name);
    }
  }
}

#[cfg(test)]
mod tests;
