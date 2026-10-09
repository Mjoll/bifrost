use std::marker::PhantomData;

use super::{
  Error,
  MemoryMappable,
  SlicerMut,
};

pub struct Shm<T: MemoryMappable> {
  shm: shared_memory::Shmem,
  name: String,
  _marker: PhantomData<T>,
}

impl<T: MemoryMappable> Shm<T> {
  pub fn new(name: &str, data: T) -> Result<Shm<T>, Error> {
    let memory_layout = data.memory_layout();

    let mut shm = shared_memory::ShmemConf::new()
      .os_id(name)
      .size(memory_layout.size)
      .create()?;

    {
      let data_mut = unsafe { shm.as_slice_mut() };
      let slicer = SlicerMut::new(data_mut);
      data.initialize(slicer)?;
    }

    Ok(Shm { shm, name: name.to_string(), _marker: Default::default() })
  }

  pub fn open(name: &str) -> Result<Shm<T>, Error> {
    let shm = shared_memory::ShmemConf::new()
      .os_id(name)
      .open()?;

    Ok(Shm { shm, name: name.to_string(), _marker: Default::default() })
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
  pub fn data_mut(&mut self) -> Result<T::Mut<'_>, Error> {
    let data_mut = unsafe { self.shm.as_slice_mut() };
    let slicer = SlicerMut::new(data_mut);
    let (_slicer, mapped) = T::map_mut(slicer)?;
    Ok(mapped)
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

