
extern crate libc;
extern crate log;
extern crate regex;
extern crate fuse;
extern crate filesystem;

use std::rc::Rc;
use std::cell::RefCell;
use std::net;
use std::io::prelude::*;
use self::fuse::{FileType};
use self::libc::consts::os::posix88::*;

use self::filesystem::*;
use self::filesystem::fs::*;
use self::filesystem::ops::*;
use self::filesystem::common::*;
use self::filesystem::core::Priority;

pub struct RootDirOps {
    installed: Vec<String>,
}

impl RootDirOps {
    pub fn new() -> RcRefBox<Operations> {
        RcRefBox!(RootDirOps { installed: Vec::new() })
    }
}

impl ops::Operations for RootDirOps {
    fn name(&self) -> &str {
        "netfs.tcp.RootDirOps"
    }

    fn install(&mut self, fs: &mut BasicFileSystem) -> bool {
        let ops = vec![ClientOps::new(), SessionDirOps::new()];
        self.installed = ops.iter().map(|v| v.borrow().name().to_owned()).collect();
        for op in ops {
            fs.register_ops(Priority::max_value(), op);
        }
        true
    }

    fn uninstall(&mut self, fs: &mut BasicFileSystem) -> bool {
        for ref ops_name in self.installed.iter().rev() {
            fs.unregister_ops(ops_name);
        }
        true
    }

    fn new_ops(&self) -> RcRefBox<Operations> {
        Self::new()
    }

    fn is_target(&mut self, path: &Path, kind: FileType) -> bool {
        kind == FileType::Directory && path == Path::new("/tcp")
    }

    fn mknod(&mut self, fs: &mut BasicFileSystem, ino: Inode, _perm: Perm) -> Result<()> {
        let dir = fs.find_node(ino).unwrap().clone();
        try!(fs.mkdir(dir.to_dir(), "1".as_ref(), 0o775));
        Ok(())
    }
}


pub struct SessionDirOps;
impl SessionDirOps {
    pub fn new() -> RcRefBox<Operations> {
        RcRefBox!(SessionDirOps)
    }
}

static SESSION_DIR_REG: regex::Regex = regex!(r"/tcp/(\d+)");

impl ops::Operations for SessionDirOps {
    fn name(&self) -> &str {
        "netfs.tcp.SessionDirOps"
    }

    fn new_ops(&self) -> RcRefBox<Operations> {
        Self::new()
    }

    fn is_target(&mut self, path: &Path, kind: FileType) -> bool {
        kind == FileType::Directory && SESSION_DIR_REG.is_match(path.to_str().unwrap())
    }
}


pub struct ClientOps {
    socket: RcRef<Option<net::TcpStream>>
}

impl ClientOps {
    fn new() -> RcRefBox<Operations> {
        RcRefBox!(ClientOps { socket: RcRef!(None) })
    }
}

static CLIENT_OPS_REG: regex::Regex = regex!(r"/tcp/((\d{1,3}\.){3}\d{1,3}:\d{1,6})");

impl Operations for ClientOps {
    fn name(&self) -> &str {
        "netfs.tcp.ClientOps"
    }

    fn new_ops(&self) -> RcRefBox<Operations> {
        Self::new()
    }

    fn is_target(&mut self, path: &Path, kind: FileType) -> bool {
        kind == FileType::RegularFile && CLIENT_OPS_REG.is_match(path.to_str().unwrap())
    }

    fn mknod(&mut self, fs: &mut BasicFileSystem, ino: Inode, _perm: Perm) -> Result<()> {
        let sockaddr: &str = &fs.find_node(ino).unwrap().name();
        let socket = try!(net::TcpStream::connect(sockaddr).or(Err(ECONNREFUSED)));
        *self.socket.borrow_mut() = Some(socket);
        Ok(())
    }

    fn rmnod(&mut self, _fs: &mut BasicFileSystem, _ino: Inode) -> Result<()> {
        Ok(())
    }

    fn open(&mut self, _fs: &mut BasicFileSystem, _ino: Inode, _perm: Perm)
        -> Result<RcRefBox<OpenHandler>>
    {
        Ok(ClientHandler::open(self.socket.clone()))
    }
}

struct ClientHandler {
    socket: RcRef<Option<net::TcpStream>>
}

impl ClientHandler {
    fn open(socket: RcRef<Option<net::TcpStream>>) -> RcRefBox<OpenHandler> {
        RcRefBox!(ClientHandler { socket: socket })
    }
}

impl OpenHandler for ClientHandler {
    fn name(&self) -> &str {
        "netfs.tcp.ClientHandler"
    }

    fn read(&mut self, _offset: u64, _size: u64) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let mut _stream = self.socket.borrow_mut();
        let stream = _stream.as_mut().unwrap();
        stream.read_to_end(&mut buf).and(Ok(buf)).or(Err(EIO))
    }

    fn write(&mut self, src: &[u8], _offset: u64, size: u64) -> Result<u64> {
        let mut _stream = self.socket.borrow_mut();
        let stream = _stream.as_mut().unwrap();
        stream.write_all(src).and(Ok(size)).or(Err(EIO))
    }

    fn release (&mut self, _flags: u32, _flush: bool) -> Result<()> {
        Ok(())
    }
}
