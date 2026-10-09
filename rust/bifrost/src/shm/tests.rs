use std::time::Duration;

use crate::{
  Mutex,
  Platform,
};

use super::{
  delete_shm,
  Error,
  FourCC,
  MemoryMappable,
  Shm,
  ShmData,
};

#[test]
fn simple_shm_test() {
  #[derive(MemoryMappable)]
  struct Payload {
    value: Mutex<Platform, u32>
  }

  impl ShmData for Payload {
    fn four_cc() -> FourCC {
      FourCC::from_static(b"pld1")
    }
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

  let payload = Payload { value: Mutex::new(2) };

  let mut shm1 : Shm<Payload> = Shm::new(shm_name, payload).unwrap();

  let timeout = Duration::from_secs(5);
  {
    let mut payload = shm1.data().unwrap();
    let lock = payload.value.lock(timeout).unwrap();
    assert_eq!(**lock, 2);
  }

  let mut shm2 : Shm<Payload> = Shm::open(shm_name).unwrap();

  {
    let mut payload = shm1.data().unwrap();
    let mut lock = payload.value.lock_mut(timeout).unwrap();
    **lock = 67;
  }

  {
    let mut payload = shm2.data().unwrap();
    let lock = payload.value.lock(timeout).unwrap();
    assert_eq!(**lock, 67);
  }

  let payload = Payload { value: Mutex::new(2) };
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

#[test]
fn simple_shm_test_2() {
  #[derive(MemoryMappable)]
  struct Payload {
    two: u32,
    one: u32,
  }

  impl ShmData for Payload {
    fn four_cc() -> FourCC {
      FourCC::from_static(b"tst1")
    }
  }

  let init = Payload {
    two: 2,
    one: 1,
  };

  let shm_name = "bifrost-test-2";

  // Clean up potential left over memory from earlier crashed runs of the
  // test
  delete_shm(shm_name).unwrap();

  {
    // The creator is the owner. We need to keep the created shm in outer scope
    // because otherwise the open will fail.
    let mut shm : Shm<Payload> = Shm::new(shm_name, init).unwrap();

    {
      let payload = shm.data().unwrap();
      assert_eq!(*(payload.two), 2);
      assert_eq!(*(payload.one), 1);
    }

    {
      let mut shm2 : Shm<Payload> = Shm::open(shm_name).unwrap();
      let payload = shm2.data().unwrap();
      assert_eq!(*(payload.two), 2);
      assert_eq!(*(payload.one), 1);
    }
  }

  match Shm::<Payload>::open(shm_name) {
    Ok(_) => {
      panic!("Expected opening of shared memory to fail after Shm has been released");
    },
    Err(Error::NotFound) => {
      // Good stuff
    },
    Err(error) => {
      panic!("Expected NotFound error, but got {:?}", error);
    }
  }
}

#[test]
fn test_shm_fourcc_check() {
  #[derive(MemoryMappable)]
  struct Payload1 {
    two: u32,
    one: u32,
  }

  impl ShmData for Payload1 {
    fn four_cc() -> FourCC {
      FourCC::from_static(b"tst1")
    }
  }

  #[derive(MemoryMappable)]
  struct Payload2 {
    three: u64,
    four: u64,
  }

  impl ShmData for Payload2 {
    fn four_cc() -> FourCC {
      FourCC::from_static(b"tst2")
    }
  }

  let init = Payload1 {
    two: 2,
    one: 1,
  };

  let shm_name = "bifrost-test-3";

  // Clean up potential left over shm from previous run
  delete_shm(shm_name).unwrap();

  let shm = Shm::new(shm_name, init).unwrap();

  match Shm::<Payload2>::open(shm_name) {
    Ok(_shm2) => {
      panic!("Expected Shm open to fail!");
    },
    Err(Error::FourCCMismatch { expected, found }) => {
      assert_eq!(expected, Payload2::four_cc());
      assert_eq!(found, Payload1::four_cc());
    },
    Err(err) => {
      panic!("Expected FourCCMismatch error, but got: {:?}", err);
    }
  }

  drop(shm);
}
