use std::time::Duration;

use clap::Parser;

use bifrost_frames::Stream;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ParsedArguments {
  stream_name: String,

  #[arg(short, long, default_value_t = 5)]
  timeout_secs: u64,
}

pub fn main() {
  let args = ParsedArguments::parse();
  let mut stream = Stream::open(&args.stream_name).unwrap();

  let timeout = Duration::from_secs(args.timeout_secs);

  while let Some(mut frame) = stream.pop(timeout).unwrap() {
    println!("Consumed frame: {}", frame.lock(timeout).unwrap().frame_number());
    stream.release_frame(frame).unwrap();
  }

  println!("End of stream");
}

