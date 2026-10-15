use clap::Parser;

use bifrost::PlatformTrait;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ParsedArgs {
  prefix: String,

  #[arg(short, long)]
  dry_run: bool,
}

pub fn main() {
  let args = ParsedArgs::parse();
  let names = bifrost::Platform::list_abandoned_shm_names(&args.prefix);
  let name_count = names.len();
  if name_count == 0 {
    println!("No abandoned shared memory detected");
    return;
  }
  println!("Abandoned shared memory:");
  for name in &names {
    let name_str = name.to_str().unwrap_or("<non-utf8>");
    println!("  {}", name_str);
  }
  if args.dry_run {
    println!("Skipping deleting since --dry-run was given");
    return;
  }

  println!("Deleting shared memory:");
  let mut deleted = 0;
  for name in names {
    let name_str = name.to_str().unwrap_or("<non-utf8>");
    if let Err(err) = bifrost::delete_shm(name) {
      println!("  Failed to delete {}: {:?}", name_str, err);
    } else {
      println!("  Deleted {}", name_str);
      deleted += 1;
    }
  }
  println!("Deleted {} shared memory entries.", deleted);
}
