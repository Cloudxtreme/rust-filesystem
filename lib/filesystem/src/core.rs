
extern crate libc;
extern crate time;
extern crate fuse;

//use std::ffi::OsStr;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use ops;
use common::*;
use fs::*;

//use self::libc::c_int;                  /* type of errno */
use self::libc::consts::os::posix88::*; /* POSIX errno */
use self::fuse::{FileType};
use self::fuse::{Request, ReplyEmpty, ReplyData, ReplyEntry, ReplyAttr};
use self::fuse::{ReplyOpen, ReplyWrite, ReplyStatfs, ReplyCreate, ReplyDirectory};

pub type Priority = u32;
pub type Handler  = u64;

pub struct BasicFileSystem {
    root: RcRef<Dir>,   // Filesystem tree
    inodes: HashMap<Inode, Node>,
    next_inode: Inode,
    ops: PriorityQueue<Priority, RcRefBox<ops::Operations>>,
    openfds: HashMap<Handler, RcRefBox<ops::OpenHandler>>,
    next_handler: Handler,
}

impl BasicFileSystem {
    pub fn new() -> BasicFileSystem {
        let dirops: RcRefBox<ops::Operations> = RcRefBox!(ops::FileOps::new());
        
        let root = RcRef!(Dir::new("/", fuse::FUSE_ROOT_ID, 0o755, dirops.clone()));
        let mut fs = BasicFileSystem {
            root: root.clone(),
            inodes: HashMap::new(),
            next_inode: 2,
            ops: PriorityQueue::new(),
            openfds: HashMap::new(),
            next_handler: 1,
        };

        fs.register_node(Node::Dir(root.clone()));
        fs.register_ops(Priority::min_value(), dirops);
        fs.register_ops(Priority::min_value(), RcRefBox!(ops::DirOps::new()));
        fs
    }

    fn get_ops(&self, path: &Path, kind: FileType) -> RcRefBox<ops::Operations> {
        // Do unwrap() due to at least default operations
        // (FileOps, DirOps) must be available at the point of the new()
        self.ops.find(|&&(_, ref t)| t.borrow_mut().is_target(path, kind)).unwrap().1.clone()
    }

    pub fn register_ops(&mut self, p: Priority, ops: RcRefBox<ops::Operations>) {
        if ops.borrow_mut().install(self) {
            self.ops.add(p, ops)
        }
    }

    pub fn unregister_ops(&mut self, name: &str) {
        let result = self.ops.remove(|&(_, ref t)| t.borrow().name() == name);
        if result.is_some() {
            let op = result.unwrap().1;
            op.borrow_mut().uninstall(self);
        }
    }

    fn register_node(&mut self, node: Node) {
        self.inodes.insert(node.attr().ino, node);
    }

    fn unregister_node(&mut self, ino: Inode) {
        self.inodes.remove(&ino);
    }

    pub fn find_node(&self, ino: Inode) -> Option<&Node> {
        self.inodes.get(&ino)
    }

    pub fn mknod(&mut self, parent_dir: &RcRef<Dir>, path: &Path, node: Node) -> Result<()> {
        parent_dir.borrow_mut().mknod(node.clone()).unwrap();   // assert if failed
        self.register_node(node.clone());

        let _ops = node.ops();
        let mut ops = _ops.borrow_mut();
        let result = ops.mknod(self, node.attr().ino, node.attr().perm);
        if result.is_err() {
            parent_dir.borrow_mut().rmnod(&node.name(), node.attr().kind);
            self.unregister_node(node.attr().ino);
        }
        result
    }

    pub fn rmnod(&mut self, _parent_dir: &RcRef<Dir>, path: &Path, kind: FileType) -> Result<()> {
        let mut parent_dir = _parent_dir.borrow_mut();
        let name = path.file_name().unwrap().to_str().unwrap();
        let result = {
            let node = try!(parent_dir.find_node(name).ok_or(ENOENT));
            let _ops = node.ops();
            let mut ops = _ops.borrow_mut();
            ops.rmnod(self, node.attr().ino)
        };
        if result.is_ok() {
            parent_dir.rmnod(name, kind).unwrap();  // assert if failed
        }
        result
    }

    pub fn mkdir(&mut self, parent_dir: &RcRef<Dir>, path: &Path, mode: u32) -> Result<RcRef<Dir>> {
        let inode = self.next_inode;
        let ops = self.get_ops(path, FileType::Directory);
        let dirname = path.file_name().unwrap().to_str().unwrap();
        let newdir = RcRef!(Dir::new(dirname, inode, mode as Perm, ops));

        match self.mknod(parent_dir, path, Node::Dir(newdir.clone())) {
            Ok(_) => { self.next_inode += 1; Ok(newdir) },
            Err(err) => Err(err)
        }
    }

    pub fn mkfile(&mut self, parent_dir: &RcRef<Dir>, path: &Path, mode: u32) -> Result<RcRef<File>> {
        let inode = self.next_inode;
        let ops = self.get_ops(path, FileType::RegularFile);
        let filename = path.file_name().unwrap().to_str().unwrap();
        let newfile = RcRef!(File::new(filename, inode, mode as Perm, ops));

        match self.mknod(parent_dir, path, Node::File(newfile.clone())) {
            Ok(_) => { self.next_inode += 1; Ok(newfile) },
            Err(err) => Err(err)
        }
    }
}

impl Drop for BasicFileSystem {
    fn drop(&mut self) {
        let names: Vec<_> = self.ops.iter().rev()
            .map(|&(_, ref t)| t.borrow().name()).collect();
        for ops_name in names {
            self.unregister_ops(&ops_name);
        }
    }
}

const TTL: time::Timespec = time::Timespec { sec: 0, nsec: 0 };

macro_rules! find_node_or_error {
    ($dir:expr, $key:expr, $reply:expr) => {
        match $dir.find_node($key) {
            Some(node) => node.clone(),
            None => { $reply.error(ENOENT); return; }
        };
    }
}

impl fuse::Filesystem for BasicFileSystem {
    fn init (&mut self, _req: &Request) -> Result<()> { Ok(()) }
    fn destroy (&mut self, _req: &Request) {}

    fn lookup (&mut self, _req: &Request, parent: Inode, name: &Path, reply: ReplyEntry) {
        println!("{}: parent={} name={:?}", "lookup", parent, name);
        let node = find_node_or_error!(self, parent, reply);
        let parent_dir = node.to_dir().borrow();
        let entry = find_node_or_error!(parent_dir, name.to_str().unwrap(), reply);
        reply.entry(&TTL, &entry.attr(), 0);
    }

    fn getattr (&mut self, _req: &Request, ino: Inode, reply: ReplyAttr) {
        println!("{}: ino={}", "getattr", ino);
        let node = find_node_or_error!(self, ino, reply);
        reply.attr(&TTL, &node.attr());
    }

    fn readdir (&mut self, _req: &Request, ino: Inode, _fh: u64, offset: u64, mut reply: ReplyDirectory) {
        println!("{}: ino={} offset={}", "readdir", ino, offset);
        let parent_dir = find_node_or_error!(self, ino, reply);
        if offset == 0 {
            reply.add(1, 0, FileType::Directory, ".");
            reply.add(1, 1, FileType::Directory, "..");
            let mut i = 2;
            for (ref name, ref node) in parent_dir.to_dir().borrow().nodes() {
                reply.add(node.attr().ino, i, node.attr().kind, name);
                i += 1;
            }
        }
        reply.ok();
    }

    fn mkdir (&mut self, _req: &Request, parent: Inode, name: &Path, mode: Mode, reply: ReplyEntry) {
        println!("{}: parent={} name={:?} mode={:o}", "mkdir", parent, name, mode);
        let parent_dir = find_node_or_error!(self, parent, reply);
        let newdir = self.mkdir(parent_dir.to_dir(), name, mode);
        match newdir {
            Ok(dir) => reply.entry(&TTL, dir.borrow().attr(), 0),
            Err(err) => reply.error(err)
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: Inode, name: &Path, reply: ReplyEmpty) {
        println!("{}: parent={} name={:?}", "rmdir", parent, name);
        let parent_dir = find_node_or_error!(self, parent, reply);
        match self.rmnod(parent_dir.to_dir(), name, FileType::Directory) {
            Ok(_) => reply.ok(),
            Err(err) => reply.error(err)
        }
    }

    fn mknod(&mut self, _req: &Request, parent: Inode, name: &Path, mode: Mode, _rdev: u32, reply: ReplyEntry) {
        println!("{}: parent={} name={:?} mode={:o}", "mknod", parent, name, mode);
        let parent_dir = find_node_or_error!(self, parent, reply);
        let newfile = self.mkfile(parent_dir.to_dir(), name, mode);
        match newfile {
            Ok(file) => reply.entry(&TTL, file.borrow().attr(), 0),
            Err(err) => reply.error(err)
        }
    }

    fn unlink(&mut self, _req: &Request, parent: Inode, name: &Path, reply: ReplyEmpty) {
        println!("{}: parent={} name={:?}", "unlink", parent, name);
        let parent_dir = find_node_or_error!(self, parent, reply);
        match self.rmnod(parent_dir.to_dir(), name, FileType::RegularFile) {
            Ok(_) => reply.ok(),
            Err(err) => reply.error(err)
        }
    }

    fn open(&mut self, _req: &Request, ino: Inode, flags: Mode, reply: ReplyOpen) {
        let node = find_node_or_error!(self, ino, reply);
        if node.attr().kind == FileType::RegularFile {
            let _ops = node.ops();
            let mut ops = _ops.borrow_mut();
            let handler = self.next_handler;
            let open_handler = match ops.open(self, ino, flags as Perm) {
                Ok(fh) => fh,
                Err(err) => { reply.error(err); return }
            };
            self.openfds.insert(handler, open_handler);
            self.next_handler += 1;
            reply.opened(handler, flags);
        } else {
            reply.error(EBADF);
        }
    }
}
