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
  SlicerMut,
};

use super::{
  Error,
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

fn pthread_mutex_memory_layout() -> MemoryLayout {
  MemoryLayout {
    alignment: mem::align_of::<libc::pthread_mutex_t>(),
    size: mem::size_of::<libc::pthread_mutex_t>(),
  }
}

pub struct LinuxPlatform;

impl PlatformTrait for LinuxPlatform {
  type Mutex<'m> = LinuxMutex<'m>;

  fn reborrow_mutex_mut<'b, 'm>(mutex: &'b mut Self::Mutex<'m>) -> Self::Mutex<'b> {
    LinuxMutex { pthread_mutex: mutex.pthread_mutex }
  }
}
