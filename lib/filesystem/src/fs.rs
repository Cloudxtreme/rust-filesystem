
extern crate libc;
extern crate time;
extern crate fuse;

use std::collections::HashMap;
use self::fuse::{FileAttr, FileType};
use self::libc::consts::os::posix88::*; /* POSIX errno */

use ops;
use common::*;

pub type Perm = u16;
pub type Mode = u32;
pub type Inode = u64;

fn fileattr_new() -> FileAttr {
    let current_time = time::get_time();
    FileAttr {
        ino: 0, size: 0,
        blocks: 0,
        atime: current_time,
        mtime: current_time,
        ctime: current_time,
        crtime: current_time,
        kind: FileType::RegularFile,
        perm: 0, nlink: 1, 
        uid: 0, gid: 0,
        rdev: 0, flags: 0,
    }
}

#[derive(Clone)]
pub enum Node {
    File(RcRef<File>),
    Dir(RcRef<Dir>),
}

impl Node {
    pub fn to_file(&self) -> &RcRef<File> {
        match self {
            &Node::File(ref file) => file,
            &Node::Dir(_) => panic!("fs::Node: cannot get a directory")
        }
    }

    pub fn to_dir(&self) -> &RcRef<Dir> {
        match self {
            &Node::File(_) => panic!("fs::Node: cannot get a file"),
            &Node::Dir(ref dir) => dir
        }
    }

    pub fn is_file(&self) -> bool {
        match self {
            &Node::File(_) => true,
            _ => false
        }
    }

    pub fn is_dir(&self) -> bool {
        !self.is_file()
    }

    pub fn name(&self) -> String {
        match self {
            &Node::File(ref file) => file.borrow().name().to_owned(),
            &Node::Dir (ref dir)  => dir.borrow().name().to_owned(),
        }.clone()
    }

    pub fn attr(&self) -> FileAttr {
        match self {
            &Node::File(ref file) => file.borrow().attr().clone(),
            &Node::Dir (ref dir)  => dir.borrow().attr().clone(),
        }
    }

    pub fn ops(&self) -> RcRefBox<ops::Operations> {
        match self {
            &Node::File(ref file) => file.borrow().ops(),
            &Node::Dir (ref dir)  => dir.borrow().ops(),
        }
    }
}

#[derive(Clone)]
pub struct File {
    name: String,
    attr: FileAttr,
    ops: RcRefBox<ops::Operations>,
}

impl File {
    pub fn new(name: &str, ino: Inode, perm: Perm, ops: RcRefBox<ops::Operations>) -> File {
        let mut attr = fileattr_new();
        attr.kind = FileType::RegularFile;
        attr.ino = ino;
        attr.perm = perm;

        File {
            name: name.to_owned(),
            attr: attr,
            ops: ops
        }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn attr(&self) -> &FileAttr { &self.attr }
    pub fn ops(&self) -> RcRefBox<ops::Operations> { self.ops.clone() }
}

#[derive(Clone)]
pub struct Dir {
    name: String,
    attr: FileAttr,
    ops: RcRefBox<ops::Operations>,
    nodes: HashMap<String, Node>,
}

impl Dir {
    pub fn new(dirname: &str, ino: u64, perm: Perm, ops: RcRefBox<ops::Operations>) -> Dir {
        let mut attr = fileattr_new();
        attr.kind = FileType::Directory;
        attr.ino = ino;
        attr.perm = perm;
        attr.nlink = 2;
        attr.size = 4096;

        Dir {
            name: dirname.to_owned(),
            attr: attr,
            ops: ops,
            nodes: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn attr(&self) -> &FileAttr { &self.attr }
    pub fn attr_mut(&mut self) -> &mut FileAttr { &mut self.attr }
    pub fn ops(&self) -> RcRefBox<ops::Operations> { self.ops.clone() }
    pub fn nodes(&self) -> &HashMap<String, Node> { &self.nodes }

    pub fn find_node(&self, name: &str) -> Option<&Node> {
        self.nodes.get(name)
    }

    pub fn mknod(&mut self, node: Node) -> Result<()> {
        let name = node.name().to_owned();
        if self.nodes.contains_key(&name) {
            Err(EEXIST)
        } else {
            self.nodes.insert(name, node);
            Ok(())
        }
    }

    pub fn rmnod(&mut self, name: &str, kind: FileType) -> Result<()> {
        let node_kind = {
            let node = try!(self.find_node(name).ok_or(ENOENT));
            node.attr().kind
        };
        if node_kind == kind {
            self.nodes.remove(name);
            Ok(())
        } else {
            Err(ENOENT)
        }
    }
}
