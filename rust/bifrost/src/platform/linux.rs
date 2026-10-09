use std::{
  mem,
  time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
  },
};

use crate::{
  MemoryLayout,
  MemoryMappable,
  SlicerMut,
};

use super::{
  Error,
  EventTrait,
  MutexTrait,
  PlatformTrait,
};


pub struct LinuxMutex<'m> {
  pthread_mutex: &'m mut libc::pthread_mutex_t,
}

pub struct LinuxLock<'l, 'm> {
  mutex: &'l mut LinuxMutex<'m>,
}

fn system_time_to_timespec(system_time: SystemTime) -> Result<libc::timespec, Error> {
  let from_epoch = system_time.duration_since(UNIX_EPOCH)?;

  Ok(libc::timespec {
    tv_sec: from_epoch.as_secs() as libc::time_t,
    tv_nsec: from_epoch.subsec_nanos() as i64,
  })
}

fn pthread_mutex_lock_helper(
  pthread_mutex: &mut libc::pthread_mutex_t,
  timeout: Duration,
) -> Result<(), Error> {
  let now = SystemTime::now();
  let absolute_timeout = now + timeout;
  let timespec = system_time_to_timespec(absolute_timeout)?;

  unsafe {
    match libc::pthread_mutex_timedlock(pthread_mutex, &timespec) {
      0 => Ok (()),
      libc::EOWNERDEAD => {
        libc::pthread_mutex_unlock(pthread_mutex);
        Err(Error::FailedToLockMutex)
      },
      libc::ETIMEDOUT => {
        Err(Error::Timeout)
      },
      _ => Err(Error::FailedToLockMutex)
    }
  }
}

impl<'l, 'm> LinuxLock<'l, 'm> {
  fn new(
    mutex: &'l mut LinuxMutex<'m>,
    duration: Duration,
  ) -> Result<LinuxLock<'l, 'm>, Error> {
    pthread_mutex_lock_helper(mutex.pthread_mutex, duration)?;
    Ok(LinuxLock { mutex })
  }
}

impl<'l, 'm> Drop for LinuxLock<'l, 'm> {
  fn drop(&mut self) {
    unsafe {
      if libc::pthread_mutex_unlock(self.mutex.pthread_mutex) != 0 {
        eprintln!("Bifrost: Failed to unlock pthread mutex");
      }
    }
  }
}

struct MutexBuilder {
  attr: libc::pthread_mutexattr_t,
}

impl MutexBuilder {
  pub fn new() -> Result<MutexBuilder, Error> {
    let attr = unsafe {
      let mut attr = mem::zeroed();
      if libc::pthread_mutexattr_init(&mut attr) != 0 {
        return Err(Error::FailedToInitializeMutex);
      }
      attr
    };
    Ok(MutexBuilder { attr })
  }

  pub fn with_pshared(mut self) -> Result<MutexBuilder, Error> {
    unsafe {
      if libc::pthread_mutexattr_setpshared(&mut self.attr, libc::PTHREAD_PROCESS_SHARED) != 0 {
        return Err(Error::FailedToInitializeMutex);
      }
    }
    Ok(self)
  }

  pub fn with_robust(mut self) -> Result<MutexBuilder, Error> {
    unsafe {
      if libc::pthread_mutexattr_setrobust(&mut self.attr, libc::PTHREAD_MUTEX_ROBUST) != 0 {
        return Err(Error::FailedToInitializeMutex);
      }
    }
    Ok(self)
  }


  pub fn build<'t>(self, dst: &'t mut [u8]) -> Result<LinuxMutex<'t>, Error> {
    let pthread_mutex : *mut libc::pthread_mutex_t =
      dst.as_mut_ptr().cast::<libc::pthread_mutex_t>();

    let init_result = unsafe {
      libc::pthread_mutex_init(pthread_mutex, &self.attr)
    };

    if init_result != 0 {
      return Err(Error::FailedToInitializeMutex);
    } else {
      return Ok(LinuxMutex { pthread_mutex: unsafe { &mut *pthread_mutex } });
    }
  }
}

impl Drop for MutexBuilder {
  fn drop(&mut self) {
    // No point to check the return code of this, if mutex attr destruction
    // fails there is nothing to be done anyway
    unsafe { libc::pthread_mutexattr_destroy(&mut self.attr) };
  }
}

pub struct LinuxLockMut<'l, 'm> {
  mutex: &'l mut LinuxMutex<'m>,
}

impl<'l, 'm> LinuxLockMut<'l, 'm> {
  fn new(
    mutex: &'l mut LinuxMutex<'m>,
    duration: Duration,
  ) -> Result<LinuxLockMut<'l, 'm>, Error> {
    pthread_mutex_lock_helper(mutex.pthread_mutex, duration)?;
    Ok(LinuxLockMut { mutex })
  }
}

impl<'l, 'm> Drop for LinuxLockMut<'l, 'm> {
  fn drop(&mut self) {
    unsafe {
      if libc::pthread_mutex_unlock(self.mutex.pthread_mutex) != 0 {
        eprintln!("Bifrost: Failed to unlock pthread mutex");
      }
    }
  }
}

impl<'m> MutexTrait<'m> for LinuxMutex<'m> {
  type Lock<'l> = LinuxLock<'l, 'm> where Self: 'l;
  type LockMut<'l> = LinuxLockMut<'l, 'm> where Self: 'l;
  fn memory_layout() -> MemoryLayout {
    pthread_mutex_memory_layout()
  }
  fn initialize<'t>(slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error> {
    let (slicer, pthread_data) = slicer.consume(pthread_mutex_memory_layout())?;

    let _mutex : LinuxMutex<'t> = MutexBuilder::new()?
      .with_pshared()?
      .with_robust()?
      .build(pthread_data)?;

    Ok(slicer)
  }
  fn from_existing_mut(slicer: SlicerMut<'m>) -> Result<(SlicerMut<'m>, Self), Error> {
    let (slicer, data) = slicer.consume(pthread_mutex_memory_layout())?;

    let pthread_mutex : &'m mut libc::pthread_mutex_t = {
      let byte_ptr : *mut u8 = data.as_mut_ptr();
      let pthread_mutex : *mut libc::pthread_mutex_t =
        byte_ptr.cast::<libc::pthread_mutex_t>();
      unsafe { &mut *pthread_mutex }
    };

    let mutex = LinuxMutex { pthread_mutex };

    Ok((slicer, mutex))
  }

  fn lock<'l>(&'l mut self, timeout: Duration) -> Result<Self::Lock<'l>, Error> {
    LinuxLock::new(self, timeout)
  }
  fn lock_mut<'l>(&'l mut self, timeout: Duration) -> Result<Self::LockMut<'l>, Error> {
    LinuxLockMut::new(self, timeout)
  }
}

pub struct LinuxEvent<'e> {
  mutex: LinuxMutex<'e>,
  pthread_cond: &'e mut libc::pthread_cond_t,
  signaled: &'e mut u32,
}

impl<'e> LinuxEvent<'e> {
  /**
   *  Utility function for locking the mutex and treating a 5 second
   *  timeout as an error. The signal lock is not meant to be held for
   *  long. If it takes more than 5 seconds to lock something has gone
   *  horribly wrong so we just map this to FailedToSignal
   */
  fn lock_with_event_error<'l>(
    mutex: &'l mut LinuxMutex<'e>
  ) -> Result<LinuxLock<'l, 'e>, Error> {
    match mutex.lock(EVENT_MUTEX_TIMEOUT) {
      Ok(lock) => Ok(lock),
      Err(Error::Timeout) => {
        return Err(Error::FailedToUpdateEventSignal);
      },
      Err(error) => {
        return Err(error);
      }
    }
  }

  fn set_signal_value(&mut self, value: u32) -> Result<(), Error> {
    let _lock = Self::lock_with_event_error(&mut self.mutex)?;

    *self.signaled = value;

    unsafe {
      if libc::pthread_cond_broadcast(self.pthread_cond) != 0 {
        return Err(Error::FailedToUpdateEventSignal);
      }
    }
    Ok(())
  }
}

struct CondBuilder {
  attr: libc::pthread_condattr_t,
}

impl CondBuilder {
  pub fn new() -> Result<CondBuilder, Error> {
    let attr : libc::pthread_condattr_t = unsafe {
      let mut attr : libc::pthread_condattr_t = mem::zeroed();


      if libc::pthread_condattr_init(&mut attr) != 0 {
        return Err(Error::FailedToInitializeEvent);
      }
      attr
    };

    Ok(CondBuilder { attr })
  }

  pub fn with_pshared(mut self) -> Result<CondBuilder, Error> {
    unsafe {
      if libc::pthread_condattr_setpshared(&mut self.attr, libc::PTHREAD_PROCESS_SHARED) != 0 {
        return Err(Error::FailedToInitializeEvent);
      }
    }
    Ok(self)
  }

  pub fn build<'t>(self, dst: &mut [u8]) -> Result<(), Error> {
    let pthread_cond : *mut libc::pthread_cond_t =
      dst.as_mut_ptr().cast::<libc::pthread_cond_t>();

    unsafe {
      if libc::pthread_cond_init(pthread_cond, &self.attr) != 0 {
        return Err(Error::FailedToInitializeEvent);
      }
    };
    Ok(())
  }
}

impl Drop for CondBuilder {
  fn drop(&mut self) {
    unsafe {
      // There really is nothing to do if exit code is non zero
      libc::pthread_condattr_destroy(&mut self.attr);
    }
  }
}



const EVENT_MUTEX_TIMEOUT : Duration = Duration::from_secs(5);

impl<'e> EventTrait<'e> for LinuxEvent<'e> {
  fn clear(&mut self) -> Result<(), Error> {
    self.set_signal_value(0)
  }
  fn signal(&mut self) -> Result<(), Error> {
    self.set_signal_value(1)
  }
  fn wait(&mut self, duration: Duration) -> Result<(), Error> {
    let lock = Self::lock_with_event_error(&mut self.mutex)?;

    let now = SystemTime::now();
    let abs_timeout = now + duration;
    let time_spec = system_time_to_timespec(abs_timeout)?;

    while *self.signaled == 0 {
      if SystemTime::now() >= abs_timeout {
        return Err(Error::Timeout);
      }
      unsafe {
        match libc::pthread_cond_timedwait(
          self.pthread_cond,
          lock.mutex.pthread_mutex,
          &time_spec,
        ) {
          0 => { },
          libc::ETIMEDOUT => {
            return Err(Error::Timeout);
          },
          _other => {
            return Err(Error::FailedToWaitForEvent);
          }
        }
      }
    }

    return Ok(())
  }

  fn memory_layout() -> MemoryLayout {
    LinuxMutex::memory_layout()
      .with_memory_layout(pthread_cond_memory_layout())
      .with::<u32>()
  }

  fn initialize<'t>(slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error> {
    let slicer = LinuxMutex::initialize(slicer)?;

    let (slicer, cond_data_mut) = slicer.consume(pthread_cond_memory_layout())?;

    let _ : () = CondBuilder::new()?
      .with_pshared()?
      .build(cond_data_mut)?;

    let signaled = 0u32;
    let slicer = signaled.initialize(slicer)?;

    Ok(slicer)
  }

  fn from_existing_mut(slicer: SlicerMut<'e>) -> Result<(SlicerMut<'e>, Self), Error> {
    let (slicer, mutex) = LinuxMutex::from_existing_mut(slicer)?;

    let (slicer, cond_data_mut) = slicer.consume(pthread_cond_memory_layout())?;

    let pthread_cond : &'e mut libc::pthread_cond_t = {
      let byte_ptr : *mut u8 = cond_data_mut.as_mut_ptr();
      let pthread_cond : *mut libc::pthread_cond_t =
        byte_ptr.cast::<libc::pthread_cond_t>();
      unsafe { &mut *pthread_cond }
    };

    let (slicer, signaled) = u32::map_mut(slicer)?;

    let event = LinuxEvent {
      mutex,
      pthread_cond,
      signaled,
    };

    Ok((slicer, event))
  }
}


fn pthread_mutex_memory_layout() -> MemoryLayout {
  MemoryLayout {
    alignment: mem::align_of::<libc::pthread_mutex_t>(),
    size: mem::size_of::<libc::pthread_mutex_t>(),
  }
}

fn pthread_cond_memory_layout() -> MemoryLayout {
  MemoryLayout {
    alignment: mem::align_of::<libc::pthread_cond_t>(),
    size: mem::size_of::<libc::pthread_cond_t>(),
  }
}

pub struct LinuxPlatform;

impl PlatformTrait for LinuxPlatform {
  type Mutex<'m> = LinuxMutex<'m>;
  type Event<'e> = LinuxEvent<'e>;

  fn reborrow_mutex_mut<'b, 'm>(mutex: &'b mut Self::Mutex<'m>) -> Self::Mutex<'b> {
    LinuxMutex { pthread_mutex: mutex.pthread_mutex }
  }

  fn reborrow_event_mut<'b, 'e>(event: &'b mut Self::Event<'e>) -> Self::Event<'b> {
    LinuxEvent {
      mutex: Self::reborrow_mutex_mut(&mut event.mutex),
      pthread_cond: event.pthread_cond,
      signaled: event.signaled,
    }
  }
}
