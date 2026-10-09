use std::sync::atomic::{
  AtomicU32,
  Ordering,
};

use bytemuck::{
  Pod,
  Zeroable,
};

use std::marker::PhantomData;

use super::{
  Error,
  FourCC,
  MemoryMappable,
  MemoryLayout,
  SlicerMut,
};

pub trait ShmData : MemoryMappable {
  fn four_cc() -> FourCC;
}

pub struct Shm<T: ShmData> {
  shm: shared_memory::Shmem,
  name: String,
  data_layout: MemoryLayout,
  _marker: PhantomData<T>,
}

impl<T: ShmData> Shm<T> {
  pub fn new(name: &str, data: T) -> Result<Shm<T>, Error> {
    let data_layout = data.memory_layout();

    let header_layout = ShmHeader::empty().memory_layout();

    let mut size = header_layout.size;
    size += data_layout.calculate_padding(size);
    size += data_layout.size;

    let mut shm = shared_memory::ShmemConf::new()
      .os_id(name)
      .size(size)
      .create()?;


    let shm_bytes_mut = unsafe { shm.as_slice_mut() };

    // Shared memory is expected to be page aligned so there should never be
    // any misalignment possible in practice, but doing a sanity check here
    assert!(shm_bytes_mut.as_mut_ptr().addr() % header_layout.alignment == 0);

    // First pass of initialization. Skip initializing the header so that it
    // remains zeroed (Guaranteed by shared_memory create)
    {
      let slicer = SlicerMut::new(shm_bytes_mut);

      // We skip writing the header here. Leaving it zeroed.
      let (slicer, _skipped) = slicer.consume(header_layout)?;

      // We need to assure that data is aligned according to its layout,
      // because of this we first use the slicer to extract another slice
      // which is correctly aligned, then created a slicer from that which
      // we then use to initialize the data.
      let (_slicer, data_mut) = slicer.consume(data_layout)?;

      let data_slicer = SlicerMut::new(data_mut);
      data.initialize(data_slicer)?;
    }

    // Second pass of initialization. Write the actual header allowing other
    // processes to open the SHM
    {
      let slicer = SlicerMut::new(shm_bytes_mut);

      if data_layout.size > u32::MAX as usize {
        return Err(Error::ValueTooLarge);
      }

      if data_layout.alignment > u32::MAX as usize {
        return Err(Error::ValueTooLarge);
      }

      let (_slicer, header) = ShmHeader::map_mut(slicer)?;

      *header.four_cc = T::four_cc();
      header.memory_layout.size = data_layout.size as u32;

      // There is a theoretical probability that the data may not have been
      // initialized even if the reader sees the updated alignment. Because of
      // this we use the alignment field as a synchronization point. If the
      // open does atomic load of alignment and it is non-zero we know
      // everything has been flushed out.
      let alignment = unsafe {
        AtomicU32::from_ptr(&mut header.memory_layout.alignment as *mut u32)
      };

      alignment.store(data_layout.alignment as u32, Ordering::Release);
    }


    Ok(Shm { shm, name: name.to_string(), data_layout, _marker: Default::default() })
  }

  pub fn open(name: &str) -> Result<Shm<T>, Error> {
    let mut shm = shared_memory::ShmemConf::new()
      .os_id(name)
      .open()?;


    let header_layout = ShmHeader::empty().memory_layout();
    let shm_bytes_mut = unsafe { shm.as_slice_mut() };

    // Shared memory is expected to be page aligned so there should never be
    // any misalignment possible in practice, but doing a sanity check here
    assert!(shm_bytes_mut.as_mut_ptr().addr() % header_layout.alignment == 0);

    let slicer = SlicerMut::new(shm_bytes_mut);
    let (_slicer, header) = ShmHeader::map_mut(slicer)?;

    let atomic_alignment = unsafe {
      AtomicU32::from_ptr(&mut header.memory_layout.alignment as *mut u32)
    };

    let alignment = atomic_alignment.load(Ordering::Acquire) as usize;

    if alignment == 0 {
      return Err(Error::NotInitialized);
    }

    // At this point the rest of the header should be Ok to read

    let data_layout = MemoryLayout {
      size: header.memory_layout.size as usize,
      alignment,
    };

    if *header.four_cc == FourCC::empty() ||
      data_layout.size == 0 ||
      data_layout.alignment == 0
    {
      return Err(Error::NotInitialized);
    }

    if *header.four_cc != T::four_cc() {
      return Err(Error::FourCCMismatch {
        expected: T::four_cc(),
        found: *header.four_cc,
      });
    }

    Ok(Shm { shm, name: name.to_string(), data_layout, _marker: Default::default() })
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
  pub fn data_mut<'t>(&'t mut self) -> Result<T::Mut<'t>, Error> {
    let slicer = SlicerMut::new(unsafe { self.shm.as_slice_mut() });

    let (slicer, _header) = ShmHeader::map_mut(slicer)?;

    // We need to assure that data is aligned according to its layout,
    // because of this we first use the slicer to extract another slice
    // which is correctly aligned, then created a slicer from that which
    // we then use to map the data.
    let (_slicer, data_bytes) = slicer.consume(self.data_layout)?;
    let data_slicer = SlicerMut::new(data_bytes);
    let (_slicer, data) = T::map_mut(data_slicer)?;

    Ok(data)
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


#[derive(Pod, Copy, Clone, Zeroable)]
#[repr(C)]
struct SerializableMemoryLayout {
  size: u32,
  alignment: u32,
}

use crate::derive::*;

#[derive(MemoryMappable)]
struct ShmHeader {
  four_cc: FourCC,
  memory_layout: SerializableMemoryLayout,
}

impl ShmHeader {
  pub fn empty() -> ShmHeader {
    ShmHeader {
      four_cc: FourCC::empty(),
      memory_layout: SerializableMemoryLayout {
        size: 0,
        alignment: 0,
      }
    }
  }
}

#[cfg(test)]
mod tests;

