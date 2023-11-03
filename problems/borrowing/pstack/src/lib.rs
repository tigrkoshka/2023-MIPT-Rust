#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;

pub struct PRef<T> {
    value: Rc<T>,
    previous: Option<Rc<RefCell<PRef<T>>>>,
}

impl<T> std::ops::Deref for PRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct PRefIter<T> {
    node: Option<Rc<RefCell<PRef<T>>>>,
}

impl<T> Iterator for PRefIter<T> {
    type Item = PRef<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr_opt = self.node.take()?;
        let curr = curr_opt.borrow();

        self.node = curr.previous.as_ref().map(Rc::clone);

        Some(PRef {
            value: Rc::clone(&curr.value),
            previous: curr.previous.as_ref().map(Rc::clone),
        })
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct PStack<T> {
    len: usize,
    head: Option<Rc<RefCell<PRef<T>>>>,
}

impl<T> Default for PStack<T> {
    fn default() -> Self {
        PStack { len: 0, head: None }
    }
}

impl<T> Clone for PStack<T> {
    fn clone(&self) -> Self {
        PStack {
            len: self.len,
            head: self.head.as_ref().map(Rc::clone),
        }
    }
}

impl<T> PStack<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, value: T) -> Self {
        PStack {
            len: self.len + 1,
            head: Some(Rc::new(RefCell::new(PRef {
                value: Rc::new(value),
                previous: self.head.as_ref().map(Rc::clone),
            }))),
        }
    }

    pub fn pop(&self) -> Option<(PRef<T>, Self)> {
        self.head.as_ref().map(|head| {
            (
                PRef {
                    value: Rc::clone(&head.borrow().value),
                    previous: Some(Rc::clone(head)),
                },
                PStack {
                    len: self.len - 1,
                    head: head.borrow().previous.as_ref().map(Rc::clone),
                },
            )
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = PRef<T>> {
        PRefIter {
            node: self.head.as_ref().map(Rc::clone),
        }
    }
}
