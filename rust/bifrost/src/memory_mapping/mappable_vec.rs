use bytemuck::Pod;

use super::{
  MemoryMappable,
  MemoryLayout,
  Error,
  SlicerMut,
};

pub struct MappableVec<T: Pod> {
  inner: Vec<T>,
}

impl<T: Pod> From<Vec<T>> for MappableVec<T> {
  fn from(inner: Vec<T>) -> Self {
    MappableVec { inner }
  }
}

impl<T: Pod> MemoryMappable for MappableVec<T> {
  type Ref<'t> = &'t [T];
  type Mut<'t> = &'t mut [T];

  fn memory_layout(&self) -> MemoryLayout {
    let mut memory_layout = MemoryLayout::new();
    memory_layout = memory_layout.with::<u32>();
    memory_layout = memory_layout.with_slice::<T>(self.inner.len());
    memory_layout
  }

  fn initialize<'t>(&self, slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error> {
    // We case usize to u32 so that it is platform independent.
    if self.inner.len() > u32::MAX as usize {
      return Err(Error::ValueTooLarge);
    }
    let size : u32 = self.inner.len() as u32;
    let slicer = size.initialize(slicer)?;
    // Since this is Pod it is safe to map then initialize from the mapped data
    let (slicer, destination) = slicer.map_slice_mut::<T>(self.inner.len())?;
    destination.copy_from_slice(&self.inner);
    Ok(slicer)
  }

  fn map_mut<'t>(slicer: SlicerMut<'t>) -> Result<(SlicerMut<'t>, Self::Mut<'t>), Error> {
    let (slicer, size) = slicer.map_mut::<u32>()?;
    let size = *size as usize;
    let (slicer, mapped) = slicer.map_slice_mut(size)?;
    Ok((slicer, mapped))
  }

  fn to_immutable<'t>(mutable: Self::Mut<'t>) -> Self::Ref<'t> { mutable }

  fn reborrow_mut<'b, 'm>(mutable: &'b mut Self::Mut<'m>) -> Self::Mut<'b> { *mutable }
}

