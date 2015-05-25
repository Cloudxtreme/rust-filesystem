
#![macro_use]

extern crate libc;

use std::path;
pub type Path = path::Path;
pub type PathBuf = path::PathBuf;

use std::result;
use self::libc::c_int;  /* type of errno */
pub type Result<T> = result::Result<T, c_int>;

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

pub type RcRef<T>       = Rc<RefCell<T>>;
pub type RcRefBox<T>    = RcRef<Box<T>>;
pub type ArcRef<T>      = Arc<Mutex<T>>;
pub type ArcRefBox<T>   = ArcRef<Box<T>>;

#[macro_export]
macro_rules! RcRef {
    ($value:expr) => {
        Rc::new(RefCell::new($value))
    }
}

#[macro_export]
macro_rules! RcRefBox {
    ($value:expr) => {
        RcRef!(Box::new($value))
    }
}

#[macro_export]
macro_rules! ArcRef {
    ($value:expr) => {
        Arc::new(Mutex::new($value))
    }
}

#[macro_export]
macro_rules! ArcRefBox {
    ($value:expr) => {
        ArcRef!(Box::new($value))
    }
}

#[macro_export]
macro_rules! set_if_some {
    ($data:expr, $opt:expr) => {
        if $opt.is_some() {
            $data = $opt.unwrap();
        }
    }
}


#[derive(Debug, Clone)]
pub struct PriorityQueue<K, T> {
    data: Vec<(K, T)>
}

use std::slice;

impl<K: Ord, T> PriorityQueue<K, T> {
    pub fn new() -> PriorityQueue<K, T> {
        PriorityQueue { data: Vec::new() }
    }

    pub fn iter(&self) -> slice::Iter<(K, T)> { self.data.iter() }

    pub fn add(&mut self, k: K, t: T) {
        self.data.push((k, t));
        self.sort();
    }

    pub fn remove<F>(&mut self, f: F) -> Option<(K, T)>
        where F: FnMut(&(K, T)) -> bool {
        self.data.iter().position(f).map(|i| self.data.remove(i))
    }

    pub fn find<F>(&self, f: F) -> Option<&(K, T)>
        where F: for<'r> FnMut(&'r &(K, T)) -> bool
    {
        self.data.iter().find(f)
    }

    fn sort(&mut self) {
        self.data.sort_by(|a, b| b.0.cmp(&a.0));
    }
}
