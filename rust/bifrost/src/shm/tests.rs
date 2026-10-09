use super::{
  delete_shm,
  Error,
  Shm,
};

#[test]
fn simple_shm_test() {
  let shm_name = "bifrost-test-1";

  delete_shm(shm_name).unwrap();

  match Shm::open(shm_name) {
    Ok(_) => {
      panic!("Expected open to fail when memory does not exist!");
    },
    Err(Error::NotFound) => { },
    Err(other) => {
      panic!("Expected open to fail with NotFound error but got: {:?}", other);
    }
  }

  let mut shm1 = Shm::new(shm_name, 1024).unwrap();

  assert_eq!(shm1.data_mut().len(), 1024);

  let mut shm2 = Shm::open(shm_name).unwrap();

  shm1.data_mut()[0] = 67;

  assert_eq!(shm2.data_mut()[0], 67);

  match Shm::new(shm_name, 5) {
    Ok(_) => {
      panic!("Expected new to fail when memory already exists");
    },
    Err(Error::AlreadyExists) => { },
    Err(other) => {
      panic!("Expected open to fail with AlreadyExists error but got: {:?}", other);
    }
  }
}

