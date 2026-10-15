use std::{
  time::Duration,
  io::{
    self,
    Read,
  },
  sync::{
    Arc,
    Mutex,
  },
  thread,
};


use clap::Parser;

use bifrost_frames::{
  AudioInfo,
  AudioFormat,
  Stream,
  Fraction,
  FieldOrder,
  FrameInfo,
  PixelFormat,
  Resolution,
  VideoInfo,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ParsedArguments {
  stream_name: String,

  #[arg(short, long, default_value_t = 5)]
  timeout_secs: u64,

  #[arg(short, long)]
  frame_count: Option<u64>,
}

pub fn main() {
  let args = ParsedArguments::parse();
  let mut stream = Stream::open(&args.stream_name).unwrap();

  let timeout = Duration::from_secs(args.timeout_secs);

  let frame_rate = Fraction {
    numerator: 25,
    denominator: 1,
  };

  let resolution = Resolution {
    width: 1920,
    height: 1080,
    pixel_aspect_ratio: Fraction::one(),
  };

  let frame_info = FrameInfo {
    frame_rate,
    video: Some(VideoInfo {
      resolution,
      pixel_format: PixelFormat::RGBA16,
      field_order: FieldOrder::Progressive,
    }),
    audio: Some(AudioInfo {
      sample_rate: 48000,
      channel_count: 2,
      format: AudioFormat::S16,
    })
  };

  let terminated = Arc::new(Mutex::new(false));

  let terminated2 = terminated.clone();
  thread::spawn(move || {
    let mut one_byte = vec![0u8; 1];
    let mut stdin = io::stdin().lock();
    stdin.read(&mut one_byte).unwrap();
    *terminated2.lock().unwrap() = true;
  });

  let mut frame_number = 0;
  while !*terminated.lock().unwrap() {
    if args.frame_count == Some(frame_number) {
      break;
    }
    let mut frame = stream.allocate(frame_info).unwrap();

    {
      let mut lock = frame.lock_mut(timeout).unwrap();
      lock.set_frame_number(frame_number);
    }

    stream.push(&mut Some(frame), timeout).unwrap();
    println!("Pushed frame: {frame_number}");
    frame_number += 1;
  }


  println!("Terminated");
}

