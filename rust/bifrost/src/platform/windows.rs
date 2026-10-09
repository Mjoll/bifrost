use std::{
  ptr,
  sync::{
    Arc,
    atomic::{
      AtomicU32,
      Ordering,
    },
  },
  time::{
    Duration,
    Instant,
    SystemTime,
    UNIX_EPOCH,
  },
};

use bytemuck::{
  Pod,
  Zeroable,
};

use windows_sys::Win32::{
  Foundation::{
    CloseHandle,
    HANDLE,
    WAIT_ABANDONED,
    WAIT_OBJECT_0,
    WAIT_TIMEOUT,
  },
  System::Threading::{
    CreateEventW,
    CreateMutexW,
    INFINITE,
    ReleaseMutex,
    ResetEvent,
    SetEvent,
    SignalObjectAndWait,
    WaitForSingleObject,
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
  Name,
  PlatformTrait,
};

/**
 *  Windows has no equivalent of a process shared pthread mutex or condition
 *  variable that can live inside shared memory. Instead the shared memory
 *  holds a unique id, and every process opens named kernel objects derived
 *  from that id.
 *
 *  A named kernel object is destroyed once the last handle to it is closed.
 *  Because of this both `initialize` and `from_existing_mut` use the Create*
 *  functions, which open the object if it exists and create it otherwise.
 *  If an object is recreated nobody held a handle to it, which means nobody
 *  held the lock or was waiting on it, so a fresh object is equivalent. All
 *  state that must survive (signaled, poisoned) lives in the shared memory
 *  itself, never only in the kernel object.
 */

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct MutexState {
  id: [u32; 4],
  /**
   *  Set when a process died while holding the mutex. Once set every lock
   *  attempt fails, matching the behavior of a robust pthread mutex which is
   *  never made consistent.
   */
  poisoned: u32,
  /**
   *  Windows mutexes are recursive. pthread mutexes are not, so this is used
   *  to detect and reject recursive locking from the same thread.
   */
  locked: u32,
}

impl MutexState {
  fn new() -> MutexState {
    static COUNTER : AtomicU32 = AtomicU32::new(0);

    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map(|d| d.as_nanos() as u64)
      .unwrap_or(0);

    MutexState {
      id: [
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
        nanos as u32,
        (nanos >> 32) as u32,
      ],
      poisoned: 0,
      locked: 0,
    }
  }

  fn kernel_object_name(&self, kind: &str) -> Vec<u16> {
    let [a, b, c, d] = self.id;
    format!("Local\\bifrost-{a:08x}{b:08x}{c:08x}{d:08x}-{kind}")
      .encode_utf16()
      .chain(std::iter::once(0))
      .collect()
  }
}

struct Handle(HANDLE);

// Kernel object handles are valid process wide and the objects themselves
// are thread safe.
unsafe impl Send for Handle { }
unsafe impl Sync for Handle { }

impl Drop for Handle {
  fn drop(&mut self) {
    unsafe { CloseHandle(self.0) };
  }
}

fn open_mutex(state: &MutexState) -> Result<Handle, Error> {
  let name = state.kernel_object_name("mutex");
  let handle = unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
  if handle.is_null() {
    return Err(Error::FailedToInitializeMutex);
  }
  Ok(Handle(handle))
}

fn open_event(state: &MutexState) -> Result<Handle, Error> {
  let name = state.kernel_object_name("event");
  // Manual reset, initially not signaled. The state of the kernel event is
  // synchronized with the shared signaled value under the event mutex.
  let handle = unsafe { CreateEventW(ptr::null(), 1, 0, name.as_ptr()) };
  if handle.is_null() {
    return Err(Error::FailedToInitializeEvent);
  }
  Ok(Handle(handle))
}

/**
 *  Convert to milliseconds rounding up, so that a sub millisecond remainder
 *  does not turn into a busy loop of zero timeouts. INFINITE is excluded
 *  since a finite timeout must stay finite.
 */
fn duration_to_millis(duration: Duration) -> u32 {
  let millis = duration.as_nanos().div_ceil(1_000_000);
  millis.min((INFINITE - 1) as u128) as u32
}

pub struct WindowsMutex<'m> {
  state: &'m mut MutexState,
  handle: Arc<Handle>,
}

impl<'m> WindowsMutex<'m> {
  fn acquire(&mut self, timeout: Duration) -> Result<(), Error> {
    match unsafe { WaitForSingleObject(self.handle.0, duration_to_millis(timeout)) } {
      WAIT_OBJECT_0 => { },
      WAIT_ABANDONED => {
        // We own the mutex, but the previous owner died while holding it so
        // the protected data can not be trusted
        self.state.poisoned = 1;
      },
      WAIT_TIMEOUT => {
        return Err(Error::Timeout);
      },
      _ => {
        return Err(Error::FailedToLockMutex);
      },
    }

    if self.state.poisoned != 0 || self.state.locked != 0 {
      // Either poisoned, or this thread already holds the mutex and the
      // wait succeeded only because Windows mutexes are recursive
      unsafe { ReleaseMutex(self.handle.0) };
      return Err(Error::FailedToLockMutex);
    }

    self.state.locked = 1;
    Ok(())
  }

  fn release(&mut self) {
    self.state.locked = 0;
    unsafe {
      if ReleaseMutex(self.handle.0) == 0 {
        eprintln!("Bifrost: Failed to release Windows mutex");
      }
    }
  }
}

pub struct WindowsLock<'l, 'm> {
  mutex: &'l mut WindowsMutex<'m>,
}

impl<'l, 'm> WindowsLock<'l, 'm> {
  fn new(
    mutex: &'l mut WindowsMutex<'m>,
    duration: Duration,
  ) -> Result<WindowsLock<'l, 'm>, Error> {
    mutex.acquire(duration)?;
    Ok(WindowsLock { mutex })
  }
}

impl<'l, 'm> Drop for WindowsLock<'l, 'm> {
  fn drop(&mut self) {
    self.mutex.release();
  }
}

pub struct WindowsLockMut<'l, 'm> {
  mutex: &'l mut WindowsMutex<'m>,
}

impl<'l, 'm> WindowsLockMut<'l, 'm> {
  fn new(
    mutex: &'l mut WindowsMutex<'m>,
    duration: Duration,
  ) -> Result<WindowsLockMut<'l, 'm>, Error> {
    mutex.acquire(duration)?;
    Ok(WindowsLockMut { mutex })
  }
}

impl<'l, 'm> Drop for WindowsLockMut<'l, 'm> {
  fn drop(&mut self) {
    self.mutex.release();
  }
}

pub struct WindowsOwnedLock<'m> {
  mutex: WindowsMutex<'m>,
}

impl<'m> WindowsOwnedLock<'m> {
  fn new(
    mut mutex: WindowsMutex<'m>,
    duration: Duration,
  ) -> Result<WindowsOwnedLock<'m>, Error> {
    mutex.acquire(duration)?;
    Ok(WindowsOwnedLock { mutex })
  }
}

impl<'m> Drop for WindowsOwnedLock<'m> {
  fn drop(&mut self) {
    self.mutex.release();
  }
}

pub struct WindowsOwnedLockMut<'m> {
  mutex: WindowsMutex<'m>,
}

impl<'m> WindowsOwnedLockMut<'m> {
  fn new(
    mut mutex: WindowsMutex<'m>,
    duration: Duration,
  ) -> Result<WindowsOwnedLockMut<'m>, Error> {
    mutex.acquire(duration)?;
    Ok(WindowsOwnedLockMut { mutex })
  }
}

impl<'m> Drop for WindowsOwnedLockMut<'m> {
  fn drop(&mut self) {
    self.mutex.release();
  }
}

impl<'m> MutexTrait<'m> for WindowsMutex<'m> {
  type Lock<'l> = WindowsLock<'l, 'm> where Self: 'l;
  type LockMut<'l> = WindowsLockMut<'l, 'm> where Self: 'l;
  type OwnedLock = WindowsOwnedLock<'m>;
  type OwnedLockMut = WindowsOwnedLockMut<'m>;

  fn memory_layout() -> MemoryLayout {
    MemoryLayout::of::<MutexState>()
  }
  fn initialize<'t>(slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error> {
    let state = MutexState::new();

    // Not strictly needed since the object is created on demand when mapped,
    // but this surfaces failures to create kernel objects early.
    let _handle = open_mutex(&state)?;

    state.initialize(slicer)
  }
  fn from_existing_mut(slicer: SlicerMut<'m>) -> Result<(SlicerMut<'m>, Self), Error> {
    let (slicer, state) = MutexState::map_mut(slicer)?;

    let handle = Arc::new(open_mutex(state)?);

    Ok((slicer, WindowsMutex { state, handle }))
  }

  fn lock<'l>(&'l mut self, timeout: Duration) -> Result<Self::Lock<'l>, Error> {
    WindowsLock::new(self, timeout)
  }
  fn lock_mut<'l>(&'l mut self, timeout: Duration) -> Result<Self::LockMut<'l>, Error> {
    WindowsLockMut::new(self, timeout)
  }
  fn to_lock(self, timeout: Duration) -> Result<Self::OwnedLock, Error> {
    WindowsOwnedLock::new(self, timeout)
  }
  fn to_lock_mut(self, timeout: Duration) -> Result<Self::OwnedLockMut, Error> {
    WindowsOwnedLockMut::new(self, timeout)
  }
}

const EVENT_MUTEX_TIMEOUT : Duration = Duration::from_secs(5);

pub struct WindowsEvent<'e> {
  mutex: WindowsMutex<'e>,
  event_handle: Arc<Handle>,
  signaled: &'e mut u32,
}

impl<'e> WindowsEvent<'e> {
  /**
   *  Utility function for locking the mutex and treating a 5 second
   *  timeout as an error. The signal lock is not meant to be held for
   *  long. If it takes more than 5 seconds to lock something has gone
   *  horribly wrong so we just map this to FailedToUpdateEventSignal
   */
  fn acquire_with_event_error(mutex: &mut WindowsMutex<'e>) -> Result<(), Error> {
    match mutex.acquire(EVENT_MUTEX_TIMEOUT) {
      Ok(()) => Ok(()),
      Err(Error::Timeout) => Err(Error::FailedToUpdateEventSignal),
      Err(error) => Err(error),
    }
  }

  fn set_signal_value(&mut self, value: u32) -> Result<(), Error> {
    Self::acquire_with_event_error(&mut self.mutex)?;

    *self.signaled = value;

    let result = unsafe {
      if value != 0 {
        SetEvent(self.event_handle.0)
      } else {
        ResetEvent(self.event_handle.0)
      }
    };

    self.mutex.release();

    if result == 0 {
      return Err(Error::FailedToUpdateEventSignal);
    }
    Ok(())
  }
}

impl<'e> EventTrait<'e> for WindowsEvent<'e> {
  fn clear(&mut self) -> Result<(), Error> {
    self.set_signal_value(0)
  }
  fn signal(&mut self) -> Result<(), Error> {
    self.set_signal_value(1)
  }
  fn wait(
    &mut self,
    duration: Duration,
    interrupt: Option<&dyn Fn() -> bool>,
  ) -> Result<(), Error> {
    let deadline = Instant::now() + duration;

    Self::acquire_with_event_error(&mut self.mutex)?;

    loop {
      // The mutex is held at the top of every iteration
      if *self.signaled != 0 {
        self.mutex.release();
        return Ok(());
      }
      if interrupt.as_ref().map(|i| i()).unwrap_or(false) {
        self.mutex.release();
        return Err(Error::Interrupted);
      }
      let now = Instant::now();
      if now >= deadline {
        self.mutex.release();
        return Err(Error::Timeout);
      }

      // The kernel event may be stale if it was recreated, the shared
      // signaled value is the source of truth. Since signal and clear both
      // update the kernel event under the mutex this keeps them in sync.
      if unsafe { ResetEvent(self.event_handle.0) } == 0 {
        self.mutex.release();
        return Err(Error::FailedToWaitForEvent);
      }

      // Atomically release the mutex and wait on the event, same as
      // pthread_cond_timedwait except that the mutex is not reacquired.
      self.mutex.state.locked = 0;
      let result = unsafe {
        SignalObjectAndWait(
          self.mutex.handle.0,
          self.event_handle.0,
          duration_to_millis(deadline - now),
          0,
        )
      };

      match result {
        WAIT_OBJECT_0 => { },
        WAIT_TIMEOUT => {
          return Err(Error::Timeout);
        },
        _ => {
          // If the call failed before signaling, the mutex is still held.
          // Releasing a mutex we do not own fails harmlessly.
          unsafe { ReleaseMutex(self.mutex.handle.0) };
          return Err(Error::FailedToWaitForEvent);
        },
      }

      Self::acquire_with_event_error(&mut self.mutex)?;
    }
  }

  fn memory_layout() -> MemoryLayout {
    WindowsMutex::memory_layout()
      .with::<u32>()
  }

  fn initialize<'t>(slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error> {
    let slicer = WindowsMutex::initialize(slicer)?;

    let signaled = 0u32;
    let slicer = signaled.initialize(slicer)?;

    Ok(slicer)
  }

  fn from_existing_mut(slicer: SlicerMut<'e>) -> Result<(SlicerMut<'e>, Self), Error> {
    let (slicer, mutex) = WindowsMutex::from_existing_mut(slicer)?;

    // The kernel event shares its id with the event's mutex
    let event_handle = Arc::new(open_event(mutex.state)?);

    let (slicer, signaled) = u32::map_mut(slicer)?;

    let event = WindowsEvent {
      mutex,
      event_handle,
      signaled,
    };

    Ok((slicer, event))
  }
}

pub struct WindowsPlatform;

impl PlatformTrait for WindowsPlatform {
  type Mutex<'m> = WindowsMutex<'m>;
  type Event<'e> = WindowsEvent<'e>;

  fn reborrow_mutex_mut<'b, 'm>(mutex: &'b mut Self::Mutex<'m>) -> Self::Mutex<'b> {
    WindowsMutex {
      state: mutex.state,
      handle: mutex.handle.clone(),
    }
  }

  fn reborrow_event_mut<'b, 'e>(event: &'b mut Self::Event<'e>) -> Self::Event<'b> {
    WindowsEvent {
      mutex: Self::reborrow_mutex_mut(&mut event.mutex),
      event_handle: event.event_handle.clone(),
      signaled: event.signaled,
    }
  }

  /**
   *  On Windows the shared_memory crate backs each mapping with a file in
   *  %TEMP%/shared_memory-rs. Every process using a mapping keeps that file
   *  open (and the mapping itself holds a reference to it), so if the file
   *  can be opened without sharing nobody is using it.
   */
  fn list_abandoned_shm_names(prefix: &str) -> Vec<Name> {
    use std::{
      fs::{
        self,
        OpenOptions,
      },
      os::windows::fs::OpenOptionsExt,
    };

    let mut shm_dir = std::env::temp_dir();
    shm_dir.push("shared_memory-rs");

    let Ok(shm_entries) = fs::read_dir(shm_dir) else {
      return Vec::new();
    };

    let mut abandoned = Vec::new();

    for entry in shm_entries {
      let Ok(entry) = entry else { continue };
      let Ok(metadata) = entry.metadata() else { continue; };
      if !metadata.is_file() {
        continue;
      }
      let Some(file_name) = entry.file_name().to_str().map(String::from) else {
        continue;
      };
      // Files being deleted are renamed with this suffix by shared_memory
      if !file_name.starts_with(prefix) || file_name.ends_with("_deleted") {
        continue;
      }

      let exclusive = OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .share_mode(0)
        .open(entry.path());

      // Any failure, sharing violation or otherwise, means it is either in
      // use or can not be cleaned up by us anyway
      if exclusive.is_ok() {
        if let Ok(name) = Name::new(&file_name) {
          abandoned.push(name);
        }
      }
    }

    abandoned
  }
}
