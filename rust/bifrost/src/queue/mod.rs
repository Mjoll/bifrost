use std::{
  collections::HashMap,
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

pub struct LocalCache<T: ShmData> {
  cache: HashMap<Name, Shm<T>>,
  stats: CacheStats,
}

impl<T: ShmData> LocalCache<T> {
  pub fn new() -> LocalCache<T> {
    LocalCache {
      cache: HashMap::new(),
      stats: CacheStats {
        cache_hits: 0,
        cache_misses: 0,
      },
    }
  }

  pub fn stats(&self) -> CacheStats {
    self.stats
  }
}

#[derive(Copy, Clone)]
pub struct CacheStats {
  pub cache_hits: usize,
  pub cache_misses: usize,
}

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
    cache: Option<&mut LocalCache<T>>
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

    if let Some(cache) = cache {
      cache.cache.insert(shm.name(), shm);
    }

    Ok(())
  }

  pub fn try_push(
    &mut self,
    in_shm: &mut Option<Shm<T>>,
    cache: Option<&mut LocalCache<T>>
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

    if let Some(cache) = cache {
      cache.cache.insert(shm.name(), shm);
    }

    Ok(true)
  }

  pub fn pop(
    &mut self,
    timeout: Duration,
    cache: Option<&mut LocalCache<T>>,
  ) -> Result<Option<Shm<T>>, Error> {
    let Some(name) = self.name_queue.pop(timeout)? else {
      return Ok(None);
    };


    if let Some(cache) = cache {
      if let Some(mut shm) = cache.cache.remove(&name) {
        cache.stats.cache_hits += 1;
        shm.acquire_ownership();
        return Ok(Some(shm));
      }
      cache.stats.cache_misses += 1;
    }

    let mut shm = Shm::open(name)?;
    shm.acquire_ownership();
    Ok(Some(shm))
  }

  pub fn try_pop(
    &mut self,
    cache: Option<&mut LocalCache<T>>,
  ) -> Result<Option<Shm<T>>, Error> {
    let Some(name) = self.name_queue.try_pop()? else {
      return Ok(None);
    };


    if let Some(cache) = cache {
      if let Some(mut shm) = cache.cache.remove(&name) {
        cache.stats.cache_hits += 1;
        shm.acquire_ownership();
        return Ok(Some(shm));
      }
      cache.stats.cache_misses += 1;
    }

    let mut shm = Shm::open(name)?;
    shm.acquire_ownership();
    Ok(Some(shm))
  }

  pub fn close(&mut self) -> Result<(), Error> {
    self.name_queue.close()
  }

  pub fn size(&self) -> usize {
    self.name_queue.size()
  }

  pub fn migrate(
    &mut self,
    new_name: Name,
    new_size: usize
  ) -> Result<Queue<P, T>, Error> {
    Ok(Queue {
      name_queue: self.name_queue.migrate(new_name, new_size)?,
      owner: true,
      _marker: Default::default(),
    })
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
