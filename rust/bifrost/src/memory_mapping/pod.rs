use bytemuck::Pod;

use super::{
  Error,
  MemoryMappable,
  MemoryLayout,
  SlicerMut,
};

impl<T: Pod> MemoryMappable for T {
  type Mut<'t> = &'t mut T;

  fn memory_layout(&self) -> MemoryLayout {
    MemoryLayout::of::<T>()
  }

  fn initialize<'t>(&self, slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error> {
    let slicer = slicer.write(self)?;
    Ok(slicer)
  }

  fn map_mut<'t>(slicer: SlicerMut<'t>) -> Result<(SlicerMut<'t>, Self::Mut<'t>), Error> {
    slicer.map_mut::<T>()
  }
}

