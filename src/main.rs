
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate fuse;
extern crate filesystem;
extern crate netfs;

use netfs::*;
use filesystem::core::Priority;

const FS_NAME: &'static str = "fuse-wlfs";

fn wlfs_main(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        println!("Usage: {} mountpoint", args[0]);
        return -1;
    }

    let mut fs = filesystem::BasicFileSystem::new();
    fs.register_ops(Priority::max_value(), tcp::RootDirOps::new());

    let options = format!(
        "-o,fsname={},allow_other,\
        intr,nonempty,direct_io", FS_NAME);

    info!("mount options: {}", options);

    fuse::mount(fs, &args[1], &[options.as_ref()]);
    return 0;
}

fn main() {
    env_logger::init().unwrap();
    let args = std::env::args().collect();
    let exit_code = wlfs_main(args);
    std::process::exit(exit_code);
}
