#![forbid(unsafe_code)]

use crate::ReceiveError::{Closed, Empty};
use std::{cell::RefCell, collections::vec_deque::VecDeque, fmt::Debug, rc::Rc, rc::Weak};
use thiserror::Error;

#[derive(Error, Debug)]
#[error("channel is closed")]
pub struct SendError<T> {
    pub value: T,
}

pub struct Sender<T> {
    queue: Weak<RefCell<VecDeque<T>>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        match self.queue.upgrade() {
            None => Err(SendError { value }),
            Some(rc) => {
                rc.borrow_mut().push_back(value);
                Ok(())
            }
        }
    }

    pub fn is_closed(&self) -> bool {
        self.queue.upgrade().is_none()
    }

    pub fn same_channel(&self, other: &Self) -> bool {
        self.queue.ptr_eq(&other.queue)
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            queue: self.queue.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {}
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum ReceiveError {
    #[error("channel is empty")]
    Empty,
    #[error("channel is closed")]
    Closed,
}

pub struct Receiver<T> {
    queue: Option<Rc<RefCell<VecDeque<T>>>>,
    leftovers: Option<VecDeque<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Result<T, ReceiveError> {
        if let Some(rc) = &self.queue {
            if Rc::weak_count(rc) == 0 {
                self.close()
            }
        }

        match &self.queue {
            Some(rc) => match rc.borrow_mut().pop_front() {
                Some(value) => Ok(value),
                None => Err(Empty),
            },
            None => match &mut self.leftovers {
                None => Err(Closed),
                Some(vec) => match vec.pop_front() {
                    Some(value) => Ok(value),
                    None => Err(Closed),
                },
            },
        }
    }

    pub fn close(&mut self) {
        let mut leftovers = VecDeque::new();
        if let Some(rc) = &self.queue {
            while let Some(x) = rc.borrow_mut().pop_front() {
                leftovers.push_back(x);
            }
        }
        self.queue = None;
        self.leftovers = Some(leftovers);
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.close();
    }
}

////////////////////////////////////////////////////////////////////////////////

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let rc = Rc::new(RefCell::new(VecDeque::new()));
    let sender = Sender {
        queue: Rc::downgrade(&rc),
    };
    let receiver = Receiver {
        queue: Some(rc),
        leftovers: None,
    };
    (sender, receiver)
}
