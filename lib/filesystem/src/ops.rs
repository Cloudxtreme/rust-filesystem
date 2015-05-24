
extern crate libc;
extern crate fuse;

use std::slice;
use std::rc::Rc;
use std::cell::RefCell;
use self::fuse::{FileType, FileAttr};
use self::libc::consts::os::posix88::*; /* POSIX errno */

use common::*;
use fs::*;
use core::BasicFileSystem;

pub trait Operations {
    fn name(&self) -> &str;
    fn new_ops(&self) -> RcRefBox<Operations>;
    fn install(&mut self, _fs: &mut BasicFileSystem) -> bool {
        println!("[!] {} installed", self.name());
        true
    }
    fn uninstall(&mut self, _fs: &mut BasicFileSystem) -> bool {
        println!("[!] {} installed", self.name());
        true
    }
    fn is_target(&mut self, _path: &Path, _kind: FileType) -> bool { false }
    fn getattr(&mut self, node: Node) -> Result<FileAttr> {
        Ok(node.attr())
    }
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
    fn read(&mut self, _offset: u64, _size: u64) -> Result<Vec<u8>>;
    fn write(&mut self, _data: &[u8], _offset: u64, _size: u64) -> Result<u64>;
    fn release (&mut self, _flags: u32, _flush: bool) -> Result<()>;
}

//
// File Operations
//
pub struct FileOps {
    data: RcRef<Vec<u8>>
}

impl FileOps {
    pub fn new() -> Self {
        FileOps { data: RcRef!(Vec::new()) }
    }
}

impl Operations for FileOps {
    fn name(&self) -> &str {
        "filesystem.FileOps"
    }

    fn new_ops(&self) -> RcRefBox<Operations> {
        RcRefBox!(Self::new())
    }

    fn is_target(&mut self, _path: &Path, kind: FileType) -> bool {
        kind == FileType::RegularFile
    }

    fn getattr(&mut self, node: Node) -> Result<FileAttr> {
        Ok(FileAttr {
            size: self.data.borrow().len() as u64,
            ..node.attr()
        })
    }

    fn mknod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode, _perm: Perm) -> Result<()> {
        Ok(())
    }

    fn rmnod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode) -> Result<()> {
        Ok(())
    }

    fn open(&mut self, _fs: &mut BasicFileSystem, ino: Inode, perm: Perm)
        -> Result<RcRefBox<OpenHandler>>
    {
        Ok(FileHandler::open(ino, perm, self.data.clone()))
    }
}

struct FileHandler {
    ino: Inode,
    perm: Perm,
    data: RcRef<Vec<u8>>
}

impl FileHandler {
    fn open(ino: Inode, perm: Perm, data: RcRef<Vec<u8>>) -> RcRefBox<OpenHandler> {
        RcRefBox!(FileHandler { ino: ino, perm: perm, data: data })
    }
}

impl OpenHandler for FileHandler {
    fn read(&mut self, offset: u64, size: u64) -> Result<Vec<u8>> {
        let data = self.data.borrow();
        let len = data.len() as u64;

        Ok(if offset > len {
            Vec::new()
        } else {
            let begin = offset as usize;
            let end = if offset + size < len { offset + size } else { len } as usize;
            (&data[begin .. end]).to_owned()
        })
    }

    fn write(&mut self, src: &[u8], offset: u64, size: u64) -> Result<u64> {
        let begin = offset as usize;
        let end = (offset + size) as usize;
        let mut dst = self.data.borrow_mut();

        if end > dst.len() {
            dst.resize(end, 0);
        }

        slice::bytes::copy_memory(src, &mut dst[begin..end]);
        Ok(size)
    }

    fn release (&mut self, _flags: u32, _flush: bool) -> Result<()> {
        Ok(())
    }
}

//
// Directory Operation
//
pub struct DirOps;

impl DirOps {
    pub fn new() -> Self { DirOps }
}

impl Operations for DirOps {
    fn name(&self) -> &str {
        "filesystem.DirOps"
    }

    fn new_ops(&self) -> RcRefBox<Operations> {
        RcRefBox!(Self::new())
    }

    fn is_target(&mut self, _path: &Path, kind: FileType) -> bool {
        kind == FileType::Directory
    }

    fn mknod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode, _perm: Perm) -> Result<()> {
        Ok(())
    }

    fn rmnod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode) -> Result<()> {
        Ok(())
    }
}
