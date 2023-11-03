#![forbid(unsafe_code)]

use std::collections::VecDeque;

#[derive(Default)]
pub struct MinQueue<T> {
    add_stack: VecDeque<(T, T)>,
    remove_stack: VecDeque<(T, T)>,
}

impl<T: Clone + Ord + std::fmt::Debug> MinQueue<T> {
    pub fn new() -> Self {
        MinQueue {
            add_stack: VecDeque::new(),
            remove_stack: VecDeque::new(),
        }
    }

    pub fn push(&mut self, val: T) {
        let min = match self.add_stack.back() {
            Some((_, add_min)) => {
                if val < *add_min {
                    val.clone()
                } else {
                    add_min.clone()
                }
            }
            None => val.clone(),
        };

        self.add_stack.push_back((val, min));
    }

    fn transfer_values(&mut self) {
        while let Some((add_val, _)) = self.add_stack.back() {
            let curr = add_val.clone();

            self.add_stack.pop_back();

            let min = match self.remove_stack.back() {
                Some((_, remove_min)) => {
                    if curr < *remove_min {
                        curr.clone()
                    } else {
                        remove_min.clone()
                    }
                }
                None => curr.clone(),
            };
            self.remove_stack.push_back((curr, min.clone()));
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let removed = match self.remove_stack.back() {
            Some((remove_val, _)) => remove_val.clone(),
            None => {
                self.transfer_values();

                match self.remove_stack.back() {
                    Some((remove_val, _)) => remove_val.clone(),
                    None => return None,
                }
            }
        };

        self.remove_stack.pop_back();
        Some(removed)
    }

    pub fn front(&self) -> Option<&T> {
        match self.remove_stack.back() {
            Some((val, _)) => Some(val),
            None => match self.add_stack.front() {
                Some((val, _)) => Some(val),
                None => None,
            },
        }
    }

    pub fn min(&self) -> Option<&T> {
        match self.remove_stack.back() {
            Some((_, remove_min)) => match self.add_stack.back() {
                Some((_, add_min)) => {
                    if remove_min < add_min {
                        Some(remove_min)
                    } else {
                        Some(add_min)
                    }
                }
                None => Some(remove_min),
            },
            None => match self.add_stack.back() {
                Some((_, add_min)) => Some(add_min),
                None => None,
            },
        }
    }

    pub fn len(&self) -> usize {
        self.add_stack.len() + self.remove_stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.add_stack.is_empty() && self.remove_stack.is_empty()
    }
}
