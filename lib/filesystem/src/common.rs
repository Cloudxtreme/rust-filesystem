
#![macro_use]

extern crate libc;

use std::path;
pub type Path = path::Path;

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

pub struct PriorityQueue<K, T> {
    data: Vec<(K, T)>
}

impl<K: Ord, T> PriorityQueue<K, T> {
    pub fn new() -> PriorityQueue<K, T> {
        PriorityQueue { data: Vec::new() }
    }

    pub fn add(&mut self, k: K, t: T) {
        self.data.push((k, t));
        self.data.sort_by(|a, b| a.0.cmp(&b.0));
    }

    pub fn find<P>(&self, predicate: P) -> Option<&(K, T)>
        where P: for<'r> FnMut(&'r &(K, T)) -> bool
    {
        self.data.iter().find(predicate)
    }
}
