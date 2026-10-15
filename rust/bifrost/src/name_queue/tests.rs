use std::{
  time::Duration,
  thread,
};

use crate::{
  delete_shm,
  Error,
  Name,
  NameQueue,
  Platform,
};

#[test]
fn test_name_queue() {
  let queue_name = Name::new("bifrost-test-queue-1").unwrap();

  delete_shm(queue_name).unwrap();

  let mut name_queue : NameQueue<Platform> =
    NameQueue::new(queue_name, 5).unwrap();


  fn thread_function(queue_name: Name) -> Result<Vec<Name>, Error> {
    let mut name_queue : NameQueue<Platform> =
      NameQueue::open(queue_name).unwrap();

    let timeout = Duration::from_secs(5);

    let mut result = Vec::new();
    while let Some(name) = name_queue.pop(timeout)? {
      result.push(name);
    }

    Ok(result)
  }

  let join_handle = thread::spawn(move || {
    thread_function(queue_name)
  });

  let timeout = Duration::from_secs(5);
  let name_count = 100;
  for i in 0..name_count {
    let name = Name::new(&format!("bifrost-test-queue-1-frame-{i}")).unwrap();
    name_queue.push(name, timeout).unwrap();
  }
  name_queue.close().unwrap();

  let result = join_handle.join().unwrap().unwrap();

  assert_eq!(result.len(), name_count);
  for (index, name) in result.into_iter().enumerate() {
    let expected = format!("bifrost-test-queue-1-frame-{index}");
    assert_eq!(name.to_str().unwrap(), expected);
  }

}
