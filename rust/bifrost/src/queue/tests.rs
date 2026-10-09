use std::{
  time::Duration,
  thread,
};

use crate::{
  derive::*,
  delete_shm,
  Error,
  FourCC,
  Mutex,
  Name,
  Platform,
  Shm,
  ShmData,
  Queue,
};

#[derive(MemoryMappable)]
struct Payload {
  value: Mutex<Platform, u64>,
}

impl ShmData for Payload {
  fn four_cc() -> FourCC {
    FourCC::from_static(b"pld8")
  }
}

fn thread_function(
  queue_name: Name,
  return_queue_name: Name,
) -> Result<Vec<u64>, Error> {
  let mut queue : Queue<Platform, Payload> =
    Queue::open(queue_name).unwrap();

  let mut return_queue : Queue<Platform, Payload> =
    Queue::open(return_queue_name).unwrap();

  let timeout = Duration::from_secs(5);

  let mut result = Vec::new();

  println!("Thread starting loop");
  while let Some(mut shm) = queue.pop(timeout)? {
    println!("Got Shm");
    {
      let mut payload = shm.data()?;
      let lock = payload.value.lock(timeout)?;
      result.push(**lock);
    }
    println!("Pushing onto return queue");
    if !return_queue.try_push(&mut Some(shm))? {
      println!("Shm dropped because return queue was full");
    }
    println!("Pushing onto return queue complete");
  }

  Ok(result)
}

fn main_thread_loop(
  count: usize,
  queue_id: usize,
  queue: &mut Queue<Platform, Payload>,
  return_queue: &mut Queue<Platform, Payload>,
) -> Result<(), Error> {
  let timeout = Duration::from_secs(5);
  let mut name_id = 0;
  for i in 0..count {
    let shm = match return_queue.try_pop()? {
      Some(mut shm) => {
        {
          let mut payload = shm.data()?;
          let mut lock = payload.value.lock_mut(timeout)?;
          **lock = i as u64;
        }
        shm
      },
      None => {
        let id = name_id;
        name_id += 1;
        let name_str = format!("bifrost-test-queue-{queue_id}-payload-{id}");
        let name = Name::new(&name_str)?;
        delete_shm(name)?;
        let payload = Payload {
          value: Mutex::new(i as u64)
        };
        Shm::new(name, payload)?
      }
    };

    queue.push(&mut Some(shm), timeout)?;
  }

  queue.close()?;
  Ok(())
}

fn check_result(
  count: usize,
  result: Vec<u64>,
) {
  assert_eq!(result.len(), count as usize);

  for (index, value) in result.into_iter().enumerate() {
    assert_eq!(index, value as usize);
  }
}

#[test]
fn queue_test() {

  let queue_name = Name::new("bifrost-test-queue-2").unwrap();
  let return_queue_name = Name::new("bifrost-test-return-queue-2").unwrap();

  delete_shm(queue_name).unwrap();
  delete_shm(return_queue_name).unwrap();

  let mut queue : Queue<Platform, Payload> =
    Queue::new(queue_name, 5).unwrap();

  let mut return_queue : Queue<Platform, Payload> =
    Queue::new(return_queue_name, 5).unwrap();


  let join_handle = thread::spawn(move || {
    thread_function(queue_name, return_queue_name)
  });

  let count = 100;

  main_thread_loop(
    count,
    2,
    &mut queue,
    &mut return_queue,
  ).unwrap();

  let result : Vec<u64> = join_handle.join().unwrap().unwrap();

  check_result(count, result);
}
