use std::{
  env,
  process::{
    Command,
    Stdio,
  },
  fs::File,
  io::{
    BufRead,
    BufReader,
    Write,
  },
  time::Duration,
  thread,
};

use crate::{
  derive::*,
  delete_shm,
  Error,
  FourCC,
  LocalCache,
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
  let mut cache : LocalCache<Payload> = LocalCache::new();

  let mut queue : Queue<Platform, Payload> =
    Queue::open(queue_name).unwrap();

  let mut return_queue : Queue<Platform, Payload> =
    Queue::open(return_queue_name).unwrap();

  let timeout = Duration::from_secs(5);

  let mut result = Vec::new();

  println!("Thread starting loop");
  while let Some(mut shm) = queue.pop(timeout, Some(&mut cache))? {
    println!("Got shm");
    {
      let mut payload = shm.data()?;
      let lock = payload.value.lock(timeout)?;
      result.push(**lock);
    }
    println!("Pushing onto return queue");
    if !return_queue.try_push(&mut Some(shm), Some(&mut cache))? {
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
  cache: &mut LocalCache<Payload>,
) -> Result<(), Error> {
  let timeout = Duration::from_secs(5);
  let mut name_id = 0;
  for i in 0..count {
    let shm = match return_queue.try_pop(Some(cache))? {
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

    queue.push(&mut Some(shm), timeout, Some(cache))?;
  }

  queue.close()?;
  Ok(())
}

fn check_result(
  count: usize,
  result: Vec<u64>,
  cache: &LocalCache<Payload>,
) {
  assert_eq!(result.len(), count as usize);

  for (index, value) in result.into_iter().enumerate() {
    assert_eq!(index, value as usize);
  }

  assert!(cache.stats().cache_misses < 10);
}

#[test]
fn queue_test() {

  let queue_name = Name::new("bifrost-test-queue-2").unwrap();
  let return_queue_name = Name::new("bifrost-test-return-queue-2").unwrap();

  delete_shm(queue_name).unwrap();
  delete_shm(return_queue_name).unwrap();

  let mut cache : LocalCache<Payload> = LocalCache::new();

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
    &mut cache,
  ).unwrap();

  let result : Vec<u64> = join_handle.join().unwrap().unwrap();

  check_result(count, result, &cache);
}

#[test]
fn multiprocess_queue_test() {
  let queue_name = Name::new("bifrost-test-queue-3").unwrap();
  let return_queue_name = Name::new("bifrost-test-return-queue-3").unwrap();
  let test_result_path = env::temp_dir().join("bifrost-multi-process-test-result.txt");

  let multi_process_key = "BIFROST_MULTI_PROCESS_QUEUE_TEST_CHILD";
  if let Ok(_) = env::var(multi_process_key) {
    println!("Child process starting");
    let values = thread_function(queue_name, return_queue_name).unwrap();

    let mut result_file = File::create(test_result_path).unwrap();

    for value in values {
      writeln!(&mut result_file, "{}", value).unwrap();
    }

    return;
  }

  delete_shm(queue_name).unwrap();
  delete_shm(return_queue_name).unwrap();

  let mut cache : LocalCache<Payload> = LocalCache::new();

  let mut queue : Queue<Platform, Payload> =
    Queue::new(queue_name, 5).unwrap();

  let mut return_queue : Queue<Platform, Payload> =
    Queue::new(return_queue_name, 5).unwrap();


  let executable = env::current_exe().unwrap();
  let mut child_process = Command::new(executable)
    .args(["--exact", "queue::tests::multiprocess_queue_test", "--nocapture"])
    .env(multi_process_key, "1")
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn().unwrap();

  let mut stderr = child_process.stderr.take().unwrap();
  let mut stdout = child_process.stdout.take().unwrap();

  let stderr_reader = thread::spawn(move || {
    let mut buf = Vec::new();
    use std::io::Read;
    stderr.read_to_end(&mut buf).unwrap();
    return str::from_utf8(&mut buf).unwrap().to_string();
  });

  let stdout_reader = thread::spawn(move || {
    let mut buf = Vec::new();
    use std::io::Read;
    stdout.read_to_end(&mut buf).unwrap();
    return str::from_utf8(&mut buf).unwrap().to_string();
  });

  let count = 100;

  let main_thread_result = main_thread_loop(
    count,
    3,
    &mut queue,
    &mut return_queue,
    &mut cache,
  );

  if !child_process.wait().unwrap().success() {
    let stdout = stdout_reader.join().unwrap();
    let stderr = stderr_reader.join().unwrap();
    println!("Child process stdout:\n{stdout}");
    println!("Child process stderr:\n{stderr}");
    if let Err(err) = main_thread_result {
      println!("Main process error: {:?}", err);
    }
    panic!("Child process died");
  }

  main_thread_result.unwrap();

  let result_file = File::open(test_result_path).unwrap();

  let buf_reader = BufReader::new(result_file);

  let mut result : Vec<u64> = Vec::new();
  for line in buf_reader.lines() {
    let value : u64 = line.unwrap().parse().unwrap();
    result.push(value);
  }

  check_result(
    count,
    result,
    &cache,
  );
}
