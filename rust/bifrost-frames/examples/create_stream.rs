use std::io::{
  self,
  Read,
};

use clap::Parser;

use bifrost_frames::Stream;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ParsedArguments {
  stream_name: String,

  #[arg(short, long, default_value_t = 5)]
  queue_size: usize,

  #[arg(short, long, default_value_t = 10)]
  pool_size: usize,
}

pub fn main() {
  let args = ParsedArguments::parse();
  let _stream = Stream::new(&args.stream_name, args.queue_size, args.pool_size).unwrap();

  println!("Stream created. Press enter to tear down.");
  let mut one_byte = vec![0u8; 1];
  let mut stdin = io::stdin().lock();
  stdin.read(&mut one_byte).unwrap();

}
