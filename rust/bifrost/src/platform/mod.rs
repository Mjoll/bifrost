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

/**
 *  Platform implementation of an [Event]. See [Event] for the semantics and
 *  for the rules callers must follow.
 *
 *  An implementation may take a lock of its own. The crate wide ordering is
 *  data mutex first, event internal lock second. Never acquire a data mutex
 *  while holding the event's own lock.
 */
pub trait EventTrait<'e> : Sized {
  fn signal(&mut self) -> Result<(), Error>;
  fn clear(&mut self) -> Result<(), Error>;
  fn wait(&mut self, duration: Duration) -> Result<(), Error>;
  fn memory_layout() -> MemoryLayout;
  fn initialize<'t>(slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error>;
  fn from_existing_mut(slicer: SlicerMut<'e>) -> Result<(SlicerMut<'e>, Self), Error>;
}

pub trait PlatformTrait : 'static {
  type Mutex<'m> : MutexTrait<'m>;
  type Event<'e> : EventTrait<'e>;

  fn reborrow_mutex_mut<'b, 'm>(mutex: &'b mut Self::Mutex<'m>) -> Self::Mutex<'b>;
  fn reborrow_event_mut<'b, 'e>(event: &'b mut Self::Event<'e>) -> Self::Event<'b>;
}

mod mutex;
pub use mutex::{
  Lock,
  LockMut,
  Mutex,
  MutexMut,
};

mod event;
pub use event::{
  Event,
  EventMut,
};

#[cfg(test)]
mod tests;



