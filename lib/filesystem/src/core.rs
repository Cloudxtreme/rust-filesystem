
extern crate libc;
extern crate time;
extern crate fuse;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use ops;
use common::*;
use fs::*;

use self::time::Timespec;
use self::libc::consts::os::posix88::*; /* POSIX errno */
use self::fuse::consts::*;
use self::fuse::{FileType, FileAttr};
use self::fuse::{Request, ReplyEmpty, ReplyData, ReplyEntry, ReplyAttr};
use self::fuse::{ReplyOpen, ReplyWrite, ReplyStatfs, ReplyDirectory};

pub type Handle   = u64;
pub type Priority = u32;

pub struct BasicFileSystem {
    root: RcRef<Dir>,   // Filesystem tree
    inodes: HashMap<Inode, Node>,
    next_inode: Inode,
    ops: PriorityQueue<Priority, RcRefBox<ops::Operations>>,
    openfds: HashMap<Handle, RcRefBox<ops::OpenHandler>>,
    next_handle: Handle,
}

// NOTE::
//  Do unwrap() on Path::to_str()
//  We have no choice but to panic!()

pub fn get_path(fs: &BasicFileSystem, node: &Node) -> PathBuf {
    assert!(node.parent() != Some(node.attr().ino));
    match node.parent() {
        Some(parent) => {
            // unwrap: Some(parent) must be valid
            let parent_node = fs.find_node(parent).unwrap();
            let mut path = get_path(fs, &parent_node);
            path.push(node.name());
            path
        },
        None => PathBuf::from("/"),
    }
}

impl BasicFileSystem {
    pub fn new() -> BasicFileSystem {
        let attr = FileAttr {
            ino: fuse::FUSE_ROOT_ID,
            perm: 0o755,
            ..fileattr_new()
        };
        let dirops = ops::DirOps::new();
        let root = RcRef!(Dir::new("/", attr, None, dirops.clone()));
        let mut fs = BasicFileSystem {
            root: root.clone(),
            inodes: HashMap::new(),
            next_inode: 2,
            ops: PriorityQueue::new(),
            openfds: HashMap::new(),
            next_handle: 1,
        };

        fs.register_node(Node::Dir(root));
        fs.register_ops(Priority::min_value(), dirops);
        fs.register_ops(Priority::min_value(), ops::FileOps::new());
        fs
    }

    fn get_ops(&self, path: &Path, kind: FileType) -> RcRefBox<ops::Operations> {
        // unwrap: At least default operations
        // (FileOps, DirOps) must be available after new()
        self.ops.find(|&&(_, ref t)| t.borrow_mut().is_target(path, kind)).unwrap().1.clone()
    }

    pub fn register_ops(&mut self, p: Priority, ops: RcRefBox<ops::Operations>) {
        if ops.borrow_mut().install(self) {
            info!("register_ops: {} installed", ops.borrow().name());
            self.ops.add(p, ops)
        }
    }

    pub fn unregister_ops(&mut self, name: &str) {
        let result = self.ops.remove(|&(_, ref t)| t.borrow().name() == name);
        if result.is_some() {
            // unwrap: result.is_some() == true
            let ops = result.unwrap().1;
            ops.borrow_mut().uninstall(self);
            info!("unregister_ops: {} uninstalled", ops.borrow().name());
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

    pub fn mknod(&mut self, parent_dir: &RcRef<Dir>, node: Node) -> Result<()> {
        try!(parent_dir.borrow_mut().mknod(node.clone()));
        self.register_node(node.clone());

        let _ops = node.ops();
        let mut ops = _ops.borrow_mut();
        let result = ops.mknod(self, node.attr().ino, node.attr().perm);

        if result.is_err() {
            let _ = parent_dir.borrow_mut().rmnod(&node.name(), node.attr().kind);
            self.unregister_node(node.attr().ino);
        }
        result
    }

    pub fn rmnod(&mut self, _parent_dir: &RcRef<Dir>, path: &Path, kind: FileType) -> Result<()> {
        let mut parent_dir = _parent_dir.borrow_mut();
        let name = path.to_str().unwrap();
        let (inode, result) = {
            let node = try!(parent_dir.find_node(name).ok_or(ENOENT));
            let inode = node.attr().ino;
            let _ops = node.ops();
            let mut ops = _ops.borrow_mut();
            (inode, ops.rmnod(self, inode))
        };
        if result.is_ok() {
            let _ = parent_dir.rmnod(name, kind);
            self.unregister_node(inode);
        }
        result
    }

    pub fn mkdir(&mut self, parent_dir: &RcRef<Dir>, path: &Path, mode: u32) -> Result<RcRef<Dir>> {
        let mut fullpath = get_path(self, &Node::Dir(parent_dir.clone()));
        fullpath.push(path);

        let ops = self.get_ops(&fullpath, FileType::Directory);
        let dirname = path.to_str().unwrap();
        let attr = FileAttr {
            ino: self.next_inode,
            perm: mode as Perm,
            ..fileattr_new()
        };
        let newdir = RcRef!(Dir::new(
            dirname, attr, None, ops.borrow().new_ops()
        ));
        self.next_inode += 1;

        info!("mkdir: fullpath={:?} ops={}", fullpath, ops.borrow().name());

        self.mknod(parent_dir, Node::Dir(newdir.clone())).and(Ok(newdir))
    }

    pub fn mkfile(&mut self, parent_dir: &RcRef<Dir>, path: &Path, mode: u32) -> Result<RcRef<File>> {
        let mut fullpath = get_path(self, &Node::Dir(parent_dir.clone()));
        fullpath.push(path);

        let ops = self.get_ops(&fullpath, FileType::RegularFile);
        let filename = path.to_str().unwrap();
        let attr = FileAttr {
            ino: self.next_inode,
            perm: mode as Perm,
            ..fileattr_new()
        };
        let newfile = RcRef!(File::new(
            filename, attr, None, ops.borrow().new_ops()
        ));
        self.next_inode += 1;

        info!("mkfile: fullpath={:?} ops={}", fullpath, ops.borrow().name());

        self.mknod(parent_dir, Node::File(newfile.clone())).and(Ok(newfile))
    }
}

impl Drop for BasicFileSystem {
    fn drop(&mut self) {
        let names: Vec<_> = self.ops.iter()
            .map(|&(_, ref t)| t.borrow().name().to_owned()).collect();
        for ref ops_name in names {
            self.unregister_ops(ops_name);
        }
    }
}

const TTL: Timespec = Timespec { sec: 0, nsec: 0 };

macro_rules! find_node_or_error {
    ($dir:expr, $key:expr, $reply:expr) => {
        match $dir.find_node($key) {
            Some(node) => node.clone(),
            None => { $reply.error(ENOENT); return; }
        }
    }
}

macro_rules! get_handler_for {
    ($fs:expr, $fh:expr, $reply:expr) => {
        match $fs.openfds.get(&$fh) {
            Some(handler) => handler,
            None => { $reply.error(EBADF); return; }
        }
    }
}

impl fuse::Filesystem for BasicFileSystem {
    fn init (&mut self, _req: &Request) -> Result<()> { Ok(()) }
    fn destroy (&mut self, _req: &Request) {}

    fn lookup (&mut self, _req: &Request, parent: Inode, name: &Path, reply: ReplyEntry) {
        let node = find_node_or_error!(self, parent, reply);
        let parent_dir = node.to_dir().borrow();
        let entry = find_node_or_error!(parent_dir, name.to_str().unwrap(), reply);
        reply.entry(&TTL, &entry.attr(), 0);
    }

    fn getattr (&mut self, _req: &Request, ino: Inode, reply: ReplyAttr) {
        let node = find_node_or_error!(self, ino, reply);
        let _ops = node.ops();
        let mut ops = _ops.borrow_mut();
        match ops.getattr(node) {
            Ok(ref attr) => reply.attr(&TTL, attr),
            Err(err) => reply.error(err)
        }
    }

    fn setattr (&mut self, _req: &Request, ino: u64, mode: Option<u32>, uid: Option<u32>, gid: Option<u32>,
        size: Option<u64>, atime: Option<Timespec>, mtime: Option<Timespec>, _fh: Option<u64>, crtime: Option<Timespec>,
        chgtime: Option<Timespec>, _bkuptime: Option<Timespec>, flags: Option<u32>, reply: ReplyAttr) {
        let mut node = find_node_or_error!(self, ino, reply);
        let mut attr = node.attr();

        set_if_some!(attr.size, size);
        set_if_some!(attr.atime, atime);
        set_if_some!(attr.mtime, mtime);
        set_if_some!(attr.ctime, chgtime);
        set_if_some!(attr.crtime, crtime);
        set_if_some!(attr.perm, mode.map(|n| n as u16));
        set_if_some!(attr.uid, uid);
        set_if_some!(attr.gid, gid);
        set_if_some!(attr.flags, flags);

        reply.attr(&TTL, &attr);
        node.set_attr(attr);
    }

    fn readdir (&mut self, _req: &Request, ino: Inode, _fh: u64, offset: u64, mut reply: ReplyDirectory) {
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
        let parent_dir = find_node_or_error!(self, parent, reply);
        let newdir = self.mkdir(parent_dir.to_dir(), name, mode);
        match newdir {
            Ok(dir) => reply.entry(&TTL, dir.borrow().attr(), 0),
            Err(err) => reply.error(err)
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: Inode, name: &Path, reply: ReplyEmpty) {
        let parent_dir = find_node_or_error!(self, parent, reply);
        match self.rmnod(parent_dir.to_dir(), name, FileType::Directory) {
            Ok(_) => reply.ok(),
            Err(err) => reply.error(err)
        }
    }

    fn mknod(&mut self, _req: &Request, parent: Inode, name: &Path, mode: Mode, _rdev: u32, reply: ReplyEntry) {
        let parent_dir = find_node_or_error!(self, parent, reply);
        let newfile = self.mkfile(parent_dir.to_dir(), name, mode);
        match newfile {
            Ok(file) => reply.entry(&TTL, file.borrow().attr(), 0),
            Err(err) => reply.error(err)
        }
    }

    fn unlink(&mut self, _req: &Request, parent: Inode, name: &Path, reply: ReplyEmpty) {
        let parent_dir = find_node_or_error!(self, parent, reply);
        match self.rmnod(parent_dir.to_dir(), name, FileType::RegularFile) {
            Ok(_) => reply.ok(),
            Err(err) => reply.error(err)
        }
    }

    fn rename(&mut self, _req: &Request, parent: u64, name: &Path, newparent: u64, newname: &Path, reply: ReplyEmpty) {
        let name = name.to_str().unwrap();
        let newname = newname.to_str().unwrap();
        let parent_dir = find_node_or_error!(self, parent, reply);
        let new_parent_dir = find_node_or_error!(self, newparent, reply);
        let mut node = find_node_or_error!(parent_dir.to_dir().borrow(), name, reply);

        node.set_name(newname);
        let _ = parent_dir.to_dir().borrow_mut().rmnod(name, node.attr().kind);
        let _ = new_parent_dir.to_dir().borrow_mut().mknod(node);

        reply.ok();
    }

    fn open(&mut self, _req: &Request, ino: Inode, flags: Mode, reply: ReplyOpen) {
        let node = find_node_or_error!(self, ino, reply);
        if node.attr().kind != FileType::Directory {
            let handle = self.next_handle;

            let _ops = node.ops();
            let mut ops = _ops.borrow_mut();
            let handler = match ops.open(self, ino, flags as Perm) {
                Ok(fh) => fh,
                Err(err) => { reply.error(err); return }
            };

            info!("open: fullpath={:?} handle={} handler={}",
                get_path(self, &node), handle, handler.borrow().name());

            self.openfds.insert(handle, handler);
            self.next_handle += 1;
            reply.opened(handle, flags | FOPEN_DIRECT_IO);
        } else {
            reply.error(EBADF);
        }
    }

    fn read (&mut self, _req: &Request, _ino: u64, fh: u64, offset: u64, size: u32, reply: ReplyData) {
        let _handler = get_handler_for!(self, fh, reply);
        let mut handler = _handler.borrow_mut();
        match handler.read(offset, size as u64) {
            Ok(data) => reply.data(&data),
            Err(err) => reply.error(err)
        }
    }

    fn write (&mut self, _req: &Request, _ino: u64, fh: u64, offset: u64, data: &[u8], _flags: u32, reply: ReplyWrite) {
        let handler = get_handler_for!(self, fh, reply);
        let result = handler.borrow_mut().write(data, offset, data.len() as u64);
        match result {
            Ok(size) => reply.written(size as u32),
            Err(err) => reply.error(err)
        }
    }

    fn release (&mut self, _req: &Request, _ino: u64, fh: u64, flags: u32, _lock_owner: u64, flush: bool, reply: ReplyEmpty) {
        let result =
            get_handler_for!(self, fh, reply).borrow_mut().release(flags, flush);
        match result {
            Ok(_) => {
                self.openfds.remove(&fh);
                reply.ok();
                info!("release: handle={}", fh);
            },
            Err(err) => reply.error(err)
        }
    }

    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        reply.ok();
    }

    fn fsync(&mut self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        reply.ok();
    }
}
