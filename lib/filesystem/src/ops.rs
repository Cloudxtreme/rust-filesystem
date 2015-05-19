
extern crate libc;
extern crate fuse;

use std::io;
use std::rc::Rc;
use std::cell::RefCell;
use self::fuse::FileType;
use self::libc::consts::os::posix88::*; /* POSIX errno */

use common::*;
use fs::*;
use core::BasicFileSystem;

pub trait Operations {
    fn name(&self) -> String;
    fn install(&mut self, _fs: &mut BasicFileSystem) -> bool { true }
    fn uninstall(&mut self, _fs: &mut BasicFileSystem) -> bool { true }
    fn is_target(&mut self, _path: &Path, _kind: FileType, ) -> bool { false }
    fn mknod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode, _mode: u32) -> Result<()> {
        Err(ENOSYS)
    }
    fn rmnod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode) -> Result<()> {
        Err(ENOSYS)
    }
    fn open(&mut self, _fs: &mut BasicFileSystem, _ino: Inode, _perm: Perm)
        -> Result<RcRefBox<OpenHandler>>
    {
        Err(ENOSYS)
    }
}

pub trait OpenHandler {
    fn read(&mut self, data: &mut [u8], offset: u64, size: u64) -> io::Result<u64>;
    fn write(&mut self, data: &[u8], offset: u64, size: u64) -> io::Result<u64>;
    fn release (&mut self, _flags: u32, _flush: bool);
}

pub struct FileOps;

impl FileOps {
    pub fn new() -> Self { FileOps }
}

impl Operations for FileOps {
    fn name(&self) -> String {
        "filesystem.FileOps".to_owned()
    }

    fn is_target(&mut self, _path: &Path, _kind: FileType, ) -> bool {
        _kind == FileType::RegularFile
    }
}

pub struct DirOps;

impl DirOps {
    pub fn new() -> Self { DirOps }
}

impl Operations for DirOps {
    fn name(&self) -> String {
        "filesystem.DirOps".to_owned()
    }

    fn is_target(&mut self, _path: &Path, _kind: FileType, ) -> bool {
        _kind == FileType::Directory
    }

    fn mknod(&mut self, fs: &mut BasicFileSystem, ino: Inode, _mode: u32) -> Result<()> {
        let node = try!(fs.find_node(ino).ok_or(ENOENT));
        println!("[!] Created directory: {}", node.name());
        Ok(())
    }

    fn rmnod(&mut self, fs: &mut BasicFileSystem, ino: Inode) -> Result<()> {
        let node = try!(fs.find_node(ino).ok_or(ENOENT));
        println!("[!] Removing directory: {}", node.name());
        Ok(())
    }
}
