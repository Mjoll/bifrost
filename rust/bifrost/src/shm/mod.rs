use super::Error;

pub struct Shm {
  shm: shared_memory::Shmem,
  name: String,
}

impl Shm {
  pub fn new(name: &str, size: usize) -> Result<Shm, Error> {
    let shm = shared_memory::ShmemConf::new()
      .os_id(name)
      .size(size)
      .create()?;

    Ok(Shm { shm, name: name.to_string() })
  }

  pub fn open(name: &str) -> Result<Shm, Error> {
    let shm = shared_memory::ShmemConf::new()
      .os_id(name)
      .open()?;

    Ok(Shm { shm, name: name.to_string() })
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn release_ownership(&mut self) {
    self.shm.set_owner(false);
  }


  pub fn acquire_ownership(&mut self) {
    self.shm.set_owner(true);
  }

  // TODO: Currently this is unsafe as shared memory is just that; shared.
  // We should not allow accessing the data without some sort of Mutex or lock.
  pub fn data_mut(&mut self) -> &mut [u8] {
    unsafe { self.shm.as_slice_mut() }
  }
}

// Delete shared memory without doing any on the content
pub fn delete_shm(name: &str) -> Result<(), Error> {
  match shared_memory::ShmemConf::new().os_id(name).open() {
    Ok(mut shm) => {
      shm.set_owner(true);
      Ok(())
    },
    Err(error) => {
      let error : Error = error.into();
      match error {
        Error::NotFound => Ok(()),
        other_error => Err(other_error)
      }
    }
  }
}

#[cfg(test)]
mod tests;

