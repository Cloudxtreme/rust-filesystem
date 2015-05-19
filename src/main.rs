
#![feature(libc)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

extern crate libc;
extern crate time;
extern crate fuse;
extern crate filesystem;

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

fn wlfs_main() -> i32 {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} mountpoint", args[0]);
        return -1;
    }

    let fs = fusefs::new("wlfs");
    fuse::mount(fs.fs, &args[1], &["allow_other".as_ref()]);

    return 0;
}

fn main() {
    std::process::exit(wlfs_main())
}
