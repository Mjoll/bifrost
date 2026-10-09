use std::time::SystemTimeError;

use super::FourCC;

#[derive(Debug)]
pub enum Error {
  NotFound,
  AlreadyExists,
  ShmemError(shared_memory::ShmemError),
  NotEnoughBytes,
  ByteMuck(bytemuck::PodCastError),
  /**
   *  In order to keep memory compatible with both 64-bit and 32-bit usize
   *  values are serialized as u32. If any of the usize values are larger
   *  than u32 MAX this error will be thrown. Places where this might be an
   *  issue:
   *
   *    Mapped vector sizes
   *    Mapped memory size
   *    Mapped memory alignment
   *
   *  In other words, quite rare to occur. But worth keeping in mind that
   *  shared memory over 4 GB may run into these issues.
   */
  ValueTooLarge,
  NotInitialized,
  FourCCMismatch { expected: FourCC, found: FourCC },
  FailedToInitializeMutex,
  FailedToLockMutex,
  Timeout,
  SystemTimeError(SystemTimeError),
}

impl From<shared_memory::ShmemError> for Error {
  fn from(value: shared_memory::ShmemError) -> Self {
    match value {
      shared_memory::ShmemError::MapOpenFailed(error) => {
        // TODO: This might be Linux specific. I have no idea if Rust have any
        // mechanism for mapping these Os numbers to specific errors.
        if error == 2 {
          Error::NotFound
        } else {
          Error::ShmemError(value)
        }
      },
      shared_memory::ShmemError::MappingIdExists => {
        Error::AlreadyExists
      },
      _ => {
        Error::ShmemError(value)
      }
    }

  }
}

impl From<bytemuck::PodCastError> for Error {
  fn from(value: bytemuck::PodCastError) -> Self {
    Error::ByteMuck(value)
  }
}

impl From<SystemTimeError> for Error {
  fn from(value: SystemTimeError) -> Self {
    Error::SystemTimeError(value)
  }
}

