use std::time::Duration;

use crate::{
  Error,
  MemoryLayout,
  MemoryMappable,
  SlicerMut,
};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::LinuxPlatform as Platform;

pub trait MutexTrait<'m> : Sized {
  type Lock<'l> where Self: 'l;
  type LockMut<'l> where Self: 'l;
  fn memory_layout() -> MemoryLayout;
  fn initialize<'t>(slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error>;
  fn from_existing_mut(slicer: SlicerMut<'m>) -> Result<(SlicerMut<'m>, Self), Error>;
  fn lock<'l>(&'l mut self, timeout: Duration) -> Result<Self::Lock<'l>, Error>;
  fn lock_mut<'l>(&'l mut self, timeout: Duration) -> Result<Self::LockMut<'l>, Error>;
}

pub trait PlatformTrait : 'static {
  type Mutex<'m> : MutexTrait<'m>;

  fn reborrow_mutex_mut<'b, 'm>(mutex: &'b mut Self::Mutex<'m>) -> Self::Mutex<'b>;
}

mod mutex;
pub use mutex::{
  Lock,
  LockMut,
  Mutex,
  MutexMut,
};

#[cfg(test)]
mod tests;



