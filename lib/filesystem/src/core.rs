
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

pub struct BasicFileSystem {
    root: RcRef<Dir>,   // Filesystem tree
    inodes: HashMap<Inode, Node>,
    next_inode: Inode,
    ops: PriorityQueue<Priority, RcRefBox<ops::Operations>>,
}

impl BasicFileSystem {
    pub fn new() -> BasicFileSystem {
        let root = RcRef!(
            Dir::new("/", fuse::FUSE_ROOT_ID, 0o755)
        );
        let mut fs = BasicFileSystem {
            root: root.clone(),
            inodes: HashMap::new(),
            next_inode: 2,
            ops: PriorityQueue::new(),
        };
        fs.register_ops(Priority::min_value(), RcRefBox!(ops::DirOps::new()));
        fs.register_dir_inode(root.clone());

        return fs;
    }

    fn get_ops(&self, path: &Path, kind: FileType) -> RcRefBox<ops::Operations> {
        let op = self.ops.find(|&&(_, ref t)| t.borrow_mut().is_target(path, kind));
        op.unwrap().1.clone()   // At least DirOps must be found
    }

    pub fn register_ops(&mut self, p: Priority, ops: RcRefBox<ops::Operations>) {
        self.ops.add(p, ops)
    }

    fn register_dir_inode(&mut self, dir: RcRef<Dir>) {
        let node = Node::Dir(dir);
        let inode = node.attr().ino;
        self.inodes.insert(inode, node);
    }

    pub fn find_node(&self, ino: Inode) -> Option<&Node> {
        self.inodes.get(&ino)
    }

    pub fn mkdir(&mut self, parent_dir: &RcRef<Dir>, path: &Path, mode: u32) -> Result<RcRef<Dir>> {
        let inode = self.next_inode;
        let dirname = path.file_name().unwrap().to_str().unwrap();
        let newdir = RcRef!(Dir::new(dirname, inode, mode as Perm));

        parent_dir.borrow_mut().mknod(Node::Dir(newdir.clone())).unwrap();
        self.register_dir_inode(newdir.clone());

        let ops = self.get_ops(path, FileType::Directory);
        let result = ops.borrow_mut().mknod(self, inode, mode);
        match result {
            Ok(_) => { self.next_inode += 1; Ok(newdir) },
            Err(err) => Err(err)
        }
    }

    pub fn rmdir(&mut self, _parent_dir: &RcRef<Dir>, path: &Path) -> Result<()> {
        let dirname = path.file_name().unwrap().to_str().unwrap();
        let mut parent_dir = _parent_dir.borrow_mut();
        let result = {
            let node = try!(parent_dir.find_node(dirname).ok_or(ENOENT));
            let _ops = self.get_ops(path, FileType::Directory);
            let mut ops = _ops.borrow_mut();
            ops.rmnod(self, node.attr().ino)
        };
        if result.is_ok() {
            parent_dir.rmnod(dirname, FileType::Directory).unwrap();
        }

        result
    }
}

const TTL: time::Timespec = time::Timespec { sec: 0, nsec: 0 };

impl fuse::Filesystem for BasicFileSystem {
    fn lookup (&mut self, _req: &Request, parent: Inode, name: &Path, reply: ReplyEntry) {
        println!("{}: parent={} name={:?}", "lookup", parent, name);
        let node = self.find_node(parent);
        if node.is_none() {
            reply.error(ENOENT);
            return;
        }
        let parent_dir = node.unwrap().to_dir().borrow();

        let node = parent_dir.find_node(name.to_str().unwrap());
        match node {
            Some(entry) => reply.entry(&TTL, &entry.attr(), 0),
            None => reply.error(ENOENT)
        }
    }

    fn getattr (&mut self, _req: &Request, ino: Inode, reply: ReplyAttr) {
        println!("{}: ino={}", "getattr", ino);
        let node = self.find_node(ino);
        match node {
            Some(entry) => reply.attr(&TTL, &entry.attr()),
            None => reply.error(ENOENT)
        }
    }

    fn readdir (&mut self, _req: &Request, ino: Inode, _fh: u64, offset: u64, mut reply: ReplyDirectory) {
        println!("{}: ino={} offset={}", "readdir", ino, offset);
        let parent_dir = match self.find_node(ino) {
            Some(node) => node.to_dir(),
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if offset == 0 {
            reply.add(1, 0, FileType::Directory, ".");
            reply.add(1, 1, FileType::Directory, "..");
            let mut i = 2;
            for (ref name, ref node) in parent_dir.borrow().nodes() {
                reply.add(node.attr().ino, i, node.attr().kind, name);
                i += 1;
            }
        }
        reply.ok();
    }

    fn mkdir (&mut self, _req: &Request, parent: Inode, name: &Path, mode: u32, reply: ReplyEntry) {
        let parent_dir = match self.find_node(parent) {
            Some(node) => node.clone(),
            None => { reply.error(ENOENT); return; }
        };

        let newdir = self.mkdir(parent_dir.to_dir(), name, mode);
        match newdir {
            Ok(dir) => reply.entry(&TTL, dir.borrow().attr(), 0),
            Err(err) => reply.error(err)
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: Inode, name: &Path, reply: ReplyEmpty) {
        let parent_dir = match self.find_node(parent) {
            Some(dir) => dir.clone(),
            None => { reply.error(ENOENT); return; }
        };

        let result = self.rmdir(parent_dir.to_dir(), name);
        match result {
            Ok(_) => reply.ok(),
            Err(err) => reply.error(err)
        }
    }
}
