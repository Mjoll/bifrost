use bytemuck::{
  Pod,
  Zeroable,
};

use super::{
  delete_shm,
  Error,
  Shm,
};

#[test]
fn simple_shm_test() {
  #[derive(Copy, Clone, Pod, Zeroable)]
  #[repr(C)]
  struct Payload {
    value: u32
  }

  let shm_name = "bifrost-test-1";

  delete_shm(shm_name).unwrap();

  match Shm::<Payload>::open(shm_name) {
    Ok(_) => {
      panic!("Expected open to fail when memory does not exist!");
    },
    Err(Error::NotFound) => { },
    Err(other) => {
      panic!("Expected open to fail with NotFound error but got: {:?}", other);
    }
  }

  let payload = Payload { value: 2 };

  let mut shm1 : Shm<Payload> = Shm::new(shm_name, payload).unwrap();

  assert_eq!(shm1.data_mut().unwrap().value, 2);

  let mut shm2 : Shm<Payload> = Shm::open(shm_name).unwrap();

  shm1.data_mut().unwrap().value = 67;

  assert_eq!(shm2.data_mut().unwrap().value, 67);

  match Shm::new(shm_name, payload) {
    Ok(_) => {
      panic!("Expected new to fail when memory already exists");
    },
    Err(Error::AlreadyExists) => { },
    Err(other) => {
      panic!("Expected open to fail with AlreadyExists error but got: {:?}", other);
    }
  }
}

