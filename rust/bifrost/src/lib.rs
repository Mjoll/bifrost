mod error;
pub use error::Error;

mod shm;
pub use shm::{
  delete_shm,
  Shm,
};
