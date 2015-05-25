
extern crate libc;
extern crate log;
extern crate fuse;
extern crate filesystem;

use std::rc::Rc;
use std::cell::RefCell;
use self::fuse::{FileType};
//use self::libc::consts::os::posix88::*;

use self::filesystem::*;
use self::filesystem::fs::*;
use self::filesystem::ops::*;
use self::filesystem::common::*;

pub struct RootDirOps;

impl RootDirOps {
    pub fn new() -> RcRefBox<Operations> {
        RcRefBox!(RootDirOps)
    }
}

impl ops::Operations for RootDirOps {
    fn name(&self) -> &str {
        "netfs.tcp.RootDirOps"
    }

    fn new_ops(&self) -> RcRefBox<Operations> {
        Self::new()
    }

    fn is_target(&mut self, path: &Path, kind: FileType) -> bool {
        kind == FileType::Directory && path == Path::new("/tcp")
    }

    fn mknod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode, _perm: Perm) -> Result<()> {
        Ok(())
    }

    fn rmnod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode) -> Result<()> {
        Ok(())
    }
}
