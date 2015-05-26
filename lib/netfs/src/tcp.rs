
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

pub struct RootDirOps;

impl RootDirOps {
    pub fn new() -> RcRefBox<Operations> {
        RcRefBox!(RootDirOps)
    }
}

impl ops::Operations for RootDirOps {
    fn name(&self) -> &str { "netfs.tcp.RootDirOps" }
    fn new_ops(&self) -> RcRefBox<Operations> { Self::new() }

    fn install(&mut self, fs: &mut BasicFileSystem) -> bool {
        fs.register_ops(Priority::max_value(), ClientOps::new());
        fs.register_ops(Priority::max_value(), SessionDirOps::new());
        fs.register_ops(Priority::max_value(), CloneOps::new());
        true
    }

    fn is_target(&mut self, path: &Path, kind: FileType) -> bool {
        kind == FileType::Directory && path == Path::new("/tcp")
    }

    fn mknod(&mut self, fs: &mut BasicFileSystem, ino: Inode, _perm: Perm) -> Result<()> {
        let dir = fs.find_node(ino).unwrap().clone();
        try!(fs.mkfile(dir.to_dir(), "clone".as_ref(), 0o660));
        Ok(())
    }
}


struct CloneOps {
    current_fd: u64
}

impl CloneOps {
    fn new() -> RcRefBox<Operations> {
        RcRefBox!(CloneOps { current_fd: 0 })
    }
}

impl Operations for CloneOps {
    fn name(&self) -> &str { "netfs.tcp.CloneOps" }
    fn new_ops(&self) -> RcRefBox<Operations> { Self::new() }

    fn is_target(&mut self, path: &Path, kind: FileType) -> bool {
        kind == FileType::RegularFile && path == Path::new("/tcp/clone")
    }

    fn open(&mut self, fs: &mut BasicFileSystem, ino: Inode, _perm: Perm)
        -> Result<RcRefBox<OpenHandler>>
    {
        let tcp_dir = {
            let node = fs.find_node(ino).unwrap();
            fs.find_node(node.parent().unwrap()).unwrap().clone()
        };

        let session: &str = &self.current_fd.to_string();
        fs.mkdir(tcp_dir.to_dir(), session.as_ref(), 0o660).and_then(|_| {
            self.current_fd += 1;
            Ok(CloneHandler::open(session))
        })
    }
}


struct CloneHandler {
    fd: String
}

impl CloneHandler {
    fn open(fd: &str) -> RcRefBox<OpenHandler> {
        RcRefBox!(CloneHandler { fd: fd.to_owned() })
    }
}

impl OpenHandler for CloneHandler {
    fn name(&self) -> &str { "netfs.tcp.CloneHandler" }

    fn read(&mut self, offset: u64, _size: u64) -> Result<Vec<u8>> {
        Ok(if offset == 0 {
            self.fd.clone().into_bytes()
        } else {
            Vec::new()
        })
    }

    fn write(&mut self, _src: &[u8], _offset: u64, _size: u64) -> Result<u64> { Err(ENOSYS) }
}


struct SessionDirOps {
    stream: Option<net::TcpStream>,
    listener: Option<net::TcpListener>,
}

impl SessionDirOps {
    pub fn new() -> RcRefBox<Operations> {
        RcRefBox!(SessionDirOps { stream: None, listener: None })
    }
}

static SESSION_DIR_REG: regex::Regex = regex!(r"/tcp/(\d+)");

impl Operations for SessionDirOps {
    fn name(&self) -> &str { "netfs.tcp.SessionDirOps" }
    fn new_ops(&self) -> RcRefBox<Operations> { Self::new() }
    fn is_target(&mut self, path: &Path, kind: FileType) -> bool {
        kind == FileType::Directory && SESSION_DIR_REG.is_match(path.to_str().unwrap())
    }
}


struct ClientOps {
    socket: Option<net::TcpStream>
}

impl ClientOps {
    fn new() -> RcRefBox<Operations> {
        RcRefBox!(ClientOps { socket: None })
    }
}

static CLIENT_OPS_REG: regex::Regex = regex!(r"/tcp/((\d{1,3}\.){3}\d{1,3}:\d{1,6})");

impl Operations for ClientOps {
    fn name(&self) -> &str { "netfs.tcp.ClientOps" }
    fn new_ops(&self) -> RcRefBox<Operations> { Self::new() }

    fn is_target(&mut self, path: &Path, kind: FileType) -> bool {
        kind == FileType::RegularFile && CLIENT_OPS_REG.is_match(path.to_str().unwrap())
    }

    fn mknod(&mut self, fs: &mut BasicFileSystem, ino: Inode, _perm: Perm) -> Result<()> {
        let sockaddr: &str = &fs.find_node(ino).unwrap().name();
        let socket = try!(net::TcpStream::connect(sockaddr).or(Err(ECONNREFUSED)));
        self.socket = Some(socket);
        Ok(())
    }

    fn open(&mut self, _fs: &mut BasicFileSystem, _ino: Inode, _perm: Perm)
        -> Result<RcRefBox<OpenHandler>>
    {
        let result = try!(self.socket.as_ref().ok_or(ENOENT).map(|s| s.try_clone()));
        Ok( ClientHandler::open( try!(result.or(Err(ENOENT))) ) )
    }
}


struct ClientHandler {
    socket: net::TcpStream
}

impl ClientHandler {
    fn open(socket: net::TcpStream) -> RcRefBox<OpenHandler> {
        RcRefBox!(ClientHandler { socket: socket })
    }
}

impl OpenHandler for ClientHandler {
    fn name(&self) -> &str { "netfs.tcp.ClientHandler" }

    fn read(&mut self, _offset: u64, _size: u64) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.socket.read_to_end(&mut buf).and(Ok(buf)).or(Err(EIO))
    }

    fn write(&mut self, src: &[u8], _offset: u64, size: u64) -> Result<u64> {
        self.socket.write_all(src).and(Ok(size)).or(Err(EIO))
    }
}
