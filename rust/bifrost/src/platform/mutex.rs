use std::{
  marker::PhantomData,
  time::Duration,
  ops::{
    Deref,
    DerefMut,
  },
};

use super::{
  Error,
  MemoryLayout,
  MemoryMappable,
  MutexTrait,
  SlicerMut,
  PlatformTrait
};

pub struct Lock<'m: 'l, 'l, P: PlatformTrait, T: MemoryMappable> {
  #[allow(dead_code)] // Lock unlocks on drop
  lock: <P::Mutex<'m> as MutexTrait<'m>>::Lock<'l>,
  data: T::Ref<'l>,
}

impl<'m, 'l, T: MemoryMappable, P: PlatformTrait> Deref for Lock<'m, 'l, P, T> {
  type Target = T::Ref<'l>;
  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

pub struct LockMut<'m: 'l, 'l, P: PlatformTrait, T: MemoryMappable> {
  #[allow(dead_code)] // Lock unlocks on drop
  lock: <P::Mutex<'m> as MutexTrait<'m>>::LockMut<'l>,
  data: &'l mut T::Mut<'m>,
}

impl<'m, 'l, T: MemoryMappable, P: PlatformTrait> Deref for LockMut<'m, 'l, P, T> {
  type Target =  T::Mut<'m>;
  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl<'m, 'l, T: MemoryMappable, P: PlatformTrait> DerefMut for LockMut<'m, 'l, P, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.data
  }
}

pub struct OwnedLock<'m, P: PlatformTrait, T: MemoryMappable> {
  #[allow(dead_code)] // Lock unlocks on drop
  lock: <P::Mutex<'m> as MutexTrait<'m>>::OwnedLock,
  data: T::Ref<'m>,
}

impl<'m, T: MemoryMappable, P: PlatformTrait> Deref for OwnedLock<'m, P, T> {
  type Target =  T::Ref<'m>;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

pub struct OwnedLockMut<'m, P: PlatformTrait, T: MemoryMappable> {
  #[allow(dead_code)] // Lock unlocks on drop
  lock: <P::Mutex<'m> as MutexTrait<'m>>::OwnedLockMut,
  data: T::Mut<'m>,
}

impl<'m, T: MemoryMappable, P: PlatformTrait> Deref for OwnedLockMut<'m, P, T> {
  type Target = T::Mut<'m>;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl<'m, T: MemoryMappable, P: PlatformTrait> DerefMut for OwnedLockMut<'m, P, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.data
  }
}

pub struct Mutex<P: PlatformTrait, T: MemoryMappable> {
  data: T,
  _marker: PhantomData<(P, T)>,
}

impl<P: PlatformTrait, T: MemoryMappable> Mutex<P, T> {
  pub fn new(data: T) -> Mutex<P, T> {
    Mutex { data, _marker: Default::default() }
  }
}

pub struct MutexMut<'m, P: PlatformTrait, T: MemoryMappable> {
  mutex: P::Mutex<'m>,
  data: T::Mut<'m>,
}

impl<'m, T: MemoryMappable, P: PlatformTrait> MutexMut<'m, P, T> {
  pub fn lock<'l>(&'l mut self, timeout: Duration) -> Result<Lock<'m, 'l, P, T>, Error> {
    let lock = Lock {
      lock: self.mutex.lock(timeout)?,
      data: T::to_immutable(T::reborrow_mut(&mut self.data)),
    };
    Ok(lock)
  }

  pub fn lock_mut<'l>(&'l mut self, timeout: Duration) -> Result<LockMut<'m, 'l, P, T>, Error> {
    let lock = LockMut {
      lock: self.mutex.lock_mut(timeout)?,
      data: &mut self.data,
    };
    Ok(lock)
  }

  pub fn to_lock(self, timeout: Duration) -> Result<OwnedLock<'m, P, T>, Error> {
    let lock = OwnedLock {
      lock: self.mutex.to_lock(timeout)?,
      data: T::to_immutable(self.data),
    };
    Ok(lock)
  }

  pub fn to_lock_mut(self, timeout: Duration) -> Result<OwnedLockMut<'m, P, T>, Error> {
    let lock = OwnedLockMut {
      lock: self.mutex.to_lock_mut(timeout)?,
      data: self.data,
    };
    Ok(lock)
  }
}

impl<T: MemoryMappable, P: PlatformTrait> MemoryMappable for Mutex<P, T> {
  type Ref<'t> = MutexMut<'t, P, T>;
  type Mut<'t> = MutexMut<'t, P, T>;

  fn memory_layout(&self) -> MemoryLayout {
    <P::Mutex<'static> as MutexTrait>::memory_layout()
      .with_memory_layout(self.data.memory_layout())
  }

  fn initialize<'t>(&self, slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error> {
    let slicer = <P::Mutex<'static> as MutexTrait>::initialize(slicer)?;

    Ok(self.data.initialize(slicer)?)
  }

  fn map_mut<'t>(slicer: SlicerMut<'t>) -> Result<(SlicerMut<'t>, Self::Mut<'t>), Error> {
    let (slicer, mutex) = <P::Mutex<'t> as MutexTrait>::from_existing_mut(slicer)?;

    let (slicer, data) = T::map_mut(slicer)?;

    let mutex = MutexMut { mutex, data };

    Ok((slicer, mutex))
  }

  fn to_immutable<'t>(mutable: Self::Mut<'t>) -> Self::Ref<'t> { mutable }

  fn reborrow_mut<'b, 'm>(mutable: &'b mut Self::Mut<'m>) -> Self::Mut<'b> {
    MutexMut {
      mutex: P::reborrow_mutex_mut(&mut mutable.mutex),
      data: T::reborrow_mut(&mut mutable.data),
    }
  }
}
