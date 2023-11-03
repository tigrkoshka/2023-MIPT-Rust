#![forbid(unsafe_code)]

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

pub struct LazyCycle<I: Iterator>
where
    I::Item: Clone,
{
    contents: Vec<I::Item>,
    idx: usize,
    done: bool,
    iter: I,
}

impl<I: Iterator> LazyCycle<I>
where
    I::Item: Clone,
{
    fn new(iter: I) -> Self {
        LazyCycle {
            contents: Vec::new(),
            idx: 0,
            done: false,
            iter,
        }
    }

    fn on_done(&mut self) -> Option<I::Item> {
        let result = self.contents.get(self.idx).cloned();

        self.idx += 1;
        if self.idx == self.contents.len() {
            self.idx = 0;
        }

        result
    }
}

impl<I: Iterator> Iterator for LazyCycle<I>
where
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return self.on_done();
        }

        match self.iter.next() {
            None => {
                self.done = true;
                self.on_done()
            }
            Some(value) => {
                let result = Some(value.clone());
                self.contents.push(value);
                result
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Extract<I: Iterator> {
    before_extracted: VecDeque<I::Item>,
    iter: I,
}

impl<I: Iterator> Extract<I> {
    fn new(mut iter: I, index: usize) -> (Option<I::Item>, Self) {
        let mut before_extracted = VecDeque::new();
        for _ in 0..index {
            match iter.next() {
                None => break,
                Some(value) => before_extracted.push_back(value),
            }
        }

        (
            iter.next(),
            Extract {
                before_extracted,
                iter,
            },
        )
    }
}

impl<I: Iterator> Iterator for Extract<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.before_extracted.is_empty() {
            return self.iter.next();
        }

        self.before_extracted.pop_front()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Tee<I: Iterator>
where
    I::Item: Clone,
{
    self_items: Rc<RefCell<VecDeque<I::Item>>>,
    other_items: Rc<RefCell<VecDeque<I::Item>>>,
    end: Rc<Cell<bool>>,
    iter: Rc<RefCell<I>>,
}

impl<I: Iterator> Tee<I>
where
    I::Item: Clone,
{
    fn new(iter: I) -> (Self, Self) {
        let first_items = Rc::new(RefCell::new(VecDeque::new()));
        let second_items = Rc::new(RefCell::new(VecDeque::new()));
        let end = Rc::new(Cell::new(false));
        let iter_rc = Rc::new(RefCell::new(iter));

        (
            Tee {
                self_items: Rc::clone(&first_items),
                other_items: Rc::clone(&second_items),
                end: Rc::clone(&end),
                iter: Rc::clone(&iter_rc),
            },
            Tee {
                self_items: second_items,
                other_items: first_items,
                end,
                iter: iter_rc,
            },
        )
    }
}

impl<I: Iterator> Iterator for Tee<I>
where
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(x) = self.self_items.borrow_mut().pop_front() {
            return Some(x);
        }

        if self.end.get() {
            return None;
        }

        match self.iter.borrow_mut().next() {
            None => {
                self.end.set(true);
                None
            }
            Some(item) => {
                self.other_items.borrow_mut().push_back(item.clone());
                Some(item)
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct GroupBy<K: Eq, I> {
    content: VecDeque<(K, Vec<I>)>,
}

impl<K: Eq, I> GroupBy<K, I> {
    fn new<It: Iterator<Item = I>, F: FnMut(&I) -> K>(iter: It, mut func: F) -> GroupBy<K, I> {
        let mut content: VecDeque<(K, Vec<I>)> = VecDeque::new();

        for item in iter {
            let key = func(&item);
            match content.back_mut() {
                None => content.push_back((key, vec![item])),
                Some((k, list)) => {
                    if k == &key {
                        list.push(item);
                    } else {
                        content.push_back((key, vec![item]));
                    }
                }
            }
        }

        GroupBy { content }
    }
}

impl<K: Eq, I> Iterator for GroupBy<K, I> {
    type Item = (K, Vec<I>);

    fn next(&mut self) -> Option<Self::Item> {
        self.content.pop_front()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait ExtendedIterator: Iterator {
    fn lazy_cycle(self) -> LazyCycle<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        LazyCycle::new(self)
    }

    fn extract(self, index: usize) -> (Option<Self::Item>, Extract<Self>)
    where
        Self: Sized,
    {
        Extract::new(self, index)
    }

    fn tee(self) -> (Tee<Self>, Tee<Self>)
    where
        Self: Sized,
        Self::Item: Clone,
    {
        Tee::new(self)
    }

    fn group_by<K: Eq, F: FnMut(&Self::Item) -> K>(self, func: F) -> GroupBy<K, Self::Item>
    where
        Self: Sized,
    {
        GroupBy::new(self, func)
    }
}

impl<I: Iterator> ExtendedIterator for I {}
