
#![feature(libc)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

extern crate libc;
extern crate time;
extern crate fuse;
extern crate filesystem;
#[macro_use]
extern crate log;
extern crate env_logger;

use filesystem::*;

struct fusefs {
    name: String,
    fs: filesystem::BasicFileSystem,
}

impl fusefs {
    fn new(fs_name: &str) -> fusefs {
        fusefs {
            name: fs_name.to_owned(),
            fs: filesystem::BasicFileSystem::new(),
        }
    }
}

impl fuse::Filesystem for fusefs {
}

fn wlfs_main(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        println!("Usage: {} mountpoint", args[0]);
        return -1;
    }

    env_logger::init().unwrap();
    let fs = fusefs::new("rust-wlfs");
    let options = format!(
        "-o,fsname={},allow_other,\
        intr,nonempty,direct_io", fs.name);

    info!("mount options: {}", options);

    fuse::mount(fs.fs, &args[1], &[options.as_ref()]);
    return 0;
}

fn main() {
    let args = std::env::args().collect();
    let exit_code = wlfs_main(args);
    std::process::exit(exit_code);
}
