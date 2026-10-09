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
  pub fn lock_mut<'l>(&'l mut self, timeout: Duration) -> Result<LockMut<'m, 'l, P, T>, Error> {
    let lock = LockMut {
      lock: self.mutex.lock_mut(timeout)?,
      data: &mut self.data,
    };
    Ok(lock)
  }
}

impl<T: MemoryMappable, P: PlatformTrait> MemoryMappable for Mutex<P, T> {
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
}
