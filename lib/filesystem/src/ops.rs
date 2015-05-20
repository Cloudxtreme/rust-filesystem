
extern crate libc;
extern crate fuse;

use std::rc::Rc;
use std::cell::RefCell;
use self::fuse::FileType;
use self::libc::consts::os::posix88::*; /* POSIX errno */

use common::*;
use fs::*;
use core::BasicFileSystem;

pub trait Operations {
    fn name(&self) -> String;
    fn install(&mut self, _fs: &mut BasicFileSystem) -> bool {
        println!("[!] {} installed", self.name());
        true
    }
    fn uninstall(&mut self, _fs: &mut BasicFileSystem) -> bool {
        println!("[!] {} installed", self.name());
        true
    }
    fn is_target(&mut self, _path: &Path, _kind: FileType, ) -> bool { false }
    fn mknod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode, _perm: Perm) -> Result<()> {
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
    fn read(&mut self, _data: &mut [u8], _offset: u64, _size: u64) -> Result<u64>;
    fn write(&mut self, _data: &[u8], _offset: u64, _size: u64) -> Result<u64>;
    fn release (&mut self, _flags: u32, _flush: bool) -> Result<()>;
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

    fn mknod(&mut self, fs: &mut BasicFileSystem, ino: Inode, _perm: Perm) -> Result<()> {
        let node = try!(fs.find_node(ino).ok_or(ENOENT));
        println!("[!] Created regular file: {}", node.name());
        Ok(())
    }

    fn rmnod(&mut self, fs: &mut BasicFileSystem, ino: Inode) -> Result<()> {
        let node = try!(fs.find_node(ino).ok_or(ENOENT));
        println!("[!] Removing regular file: {}", node.name());
        Ok(())
    }
}

struct FileHandler;

impl OpenHandler for FileHandler {
    fn read(&mut self, _data: &mut [u8], _offset: u64, _size: u64) -> Result<u64> {
        Err(ENOSYS)
    }

    fn write(&mut self, _data: &[u8], _offset: u64, _size: u64) -> Result<u64> {
        Err(ENOSYS)
    }

    fn release (&mut self, _flags: u32, _flush: bool) -> Result<()> {
        Err(ENOSYS)
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

    fn mknod(&mut self, fs: &mut BasicFileSystem, ino: Inode, _perm: Perm) -> Result<()> {
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
