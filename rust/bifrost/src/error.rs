#[derive(Debug)]
pub enum Error {
  NotFound,
  AlreadyExists,
  ShmemError(shared_memory::ShmemError),
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
