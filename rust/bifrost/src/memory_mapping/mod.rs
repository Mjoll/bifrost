mod memory_layout;
pub use memory_layout::MemoryLayout;

use super::Error;

mod slicer;
pub use slicer::{
  SlicerMut,
};

mod pod;

pub trait MemoryMappable : 'static {
  type Mut<'t>;

  fn memory_layout(&self) -> MemoryLayout;

  /**
   *  Initialized that data referenced by SlicerMut.
   *
   *  Note that the caller must ensure that the data pointed to by slicer is
   *  aligned according to what memory_layout() returns.
   **/
  fn initialize<'t>(&self, slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error>;

  fn map_mut<'t>(slicer: SlicerMut<'t>) -> Result<(SlicerMut<'t>, Self::Mut<'t>), Error>;
}

#[cfg(test)]
mod tests;
