/**
 *  Tools for slicing immutable and mutable slices into smaller chunks to be
 *  cast into proper types. The mutable variant is mostly the reason this
 *  exists as getting multiple references to a mutable array is a bit finicky.
 */

use bytemuck::Pod;

use super::{
  Error,
  MemoryLayout,
};

pub struct SlicerMut<'t> {
  remain: &'t mut [u8],
  offset: usize,
}

impl<'t> SlicerMut<'t> {
  pub fn new(data: &'t mut [u8]) -> SlicerMut<'t> {
    SlicerMut { offset: 0, remain: data }
  }

  pub fn write<T: Pod>(self, data: &T) -> Result<SlicerMut<'t>, Error> {
    let memory_layout = MemoryLayout::of::<T>();
    let (slicer, destination) = self.consume(memory_layout)?;

    let source = bytemuck::bytes_of(data);
    destination.copy_from_slice(source);

    Ok(slicer)
  }

  pub fn map_mut<T: Pod>(self) -> Result<(SlicerMut<'t>, &'t mut T), Error> {
    let memory_layout = MemoryLayout::of::<T>();
    let (slicer, data) = self.consume(memory_layout)?;
    let mapped = bytemuck::try_from_bytes_mut(data)?;
    Ok((slicer, mapped))
  }

  pub fn map_slice_mut<T: Pod>(self, count: usize) -> Result<(SlicerMut<'t>, &'t mut [T]), Error> {
    let memory_layout = MemoryLayout::of::<T>().as_slice(count);
    let (slicer, data) = self.consume(memory_layout)?;
    let mapped = bytemuck::try_cast_slice_mut(data)?;
    Ok((slicer, mapped))
  }

  pub fn consume(self, memory_layout: MemoryLayout) -> Result<(SlicerMut<'t>, &'t mut [u8]), Error> {
    let pad = memory_layout.calculate_padding(self.offset);
    let size = memory_layout.size;
    let mid = pad + size;
    if mid > self.remain.len() {
      return Err(Error::NotEnoughBytes);
    }
    let (first, second) = self.remain.split_at_mut(mid);

    let slicer = SlicerMut { remain: second, offset: self.offset + mid };

    Ok((slicer, &mut first[pad..]))
  }
}

