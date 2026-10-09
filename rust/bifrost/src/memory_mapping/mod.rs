mod memory_layout;
pub use memory_layout::MemoryLayout;

use super::Error;

mod slicer;
pub use slicer::{
  SlicerMut,
};

mod pod;
mod mappable_vec;
pub use mappable_vec::MappableVec;

pub trait MemoryMappable : 'static {
  type Ref<'t>;
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

  /**
   *  This method downgrades a mutable mapping to a immutable mapping.
   */
  fn to_immutable<'t>(mutable: Self::Mut<'t>) -> Self::Ref<'t>;

  /**
   *  This use re-borrows a Mut<'m> mapping it to lifetime 'b. Used by Mutex
   *  to be able to keep the inner data as Mut when making an immutable
   *  reference.
   */
  fn reborrow_mut<'b, 'm>(mutable: &'b mut Self::Mut<'m>) -> Self::Mut<'b>;
}

#[cfg(test)]
mod tests;
