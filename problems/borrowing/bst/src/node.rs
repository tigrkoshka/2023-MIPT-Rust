#![forbid(unsafe_code)]

use std::borrow::Borrow;
use std::cmp::{max, Ordering::*};
use std::fmt::{Display, Formatter};

pub struct Node<K, V> {
    pub(crate) key: K,
    pub(crate) value: V,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
    height: usize,
    size: usize,
}

impl<K: Ord, V> Node<K, V> {
    pub fn new(key: K, value: V) -> Self {
        Node {
            key,
            value,
            left: None,
            right: None,
            height: 1,
            size: 1,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    fn r_height(&self) -> usize {
        self.right.as_ref().map_or(0, |right| right.height)
    }

    fn l_height(&self) -> usize {
        self.left.as_ref().map_or(0, |left| left.height)
    }

    fn r_size(&self) -> usize {
        self.right.as_ref().map_or(0, |right| right.size)
    }

    fn l_size(&self) -> usize {
        self.left.as_ref().map_or(0, |left| left.size)
    }

    fn factor(&self) -> isize {
        self.r_height() as isize - self.l_height() as isize
    }

    fn r_factor(&self) -> isize {
        self.right.as_ref().map_or(0, |right| right.factor())
    }

    fn l_factor(&self) -> isize {
        self.left.as_ref().map_or(0, |left| left.factor())
    }

    fn update(&mut self) {
        self.height = max(self.r_height(), self.l_height()) + 1;
        self.size = self.r_size() + self.l_size() + 1;
    }

    fn rotate_right(&mut self) {
        /* Let's consider thw following structure:
         *
         * P:
         *   C:               <--- self
         *     L:             <--- Some
         *       LL: ...
         *       LR: ...
         *
         *     R: ...
         */

        let mut repl = self.left.take().unwrap();

        /* At this point we have:
         *
         * P:
         *   C:               <--- self
         *     R: ...
         *
         * And a detached:
         *
         * L:                 <--- repl
         *   LL: ...
         *   LR: ...
         */

        self.left = repl.right.take();

        /* At this point we have:
         *
         * P:
         *   C:               <--- self
         *     LR: ...
         *     R:  ...
         *
         * And a detached:
         *
         * L:                 <--- repl
         *   LL: ...
         */

        std::mem::swap(self, &mut *repl);

        /* At this point we have:
         *
         * P:
         *   L:               <--- self
         *     LL: ...
         *
         * And a detached:
         *
         * C:                 <--- repl
         *   LR: ...
         *   R:  ...
         */

        repl.update();
        self.right = Some(repl);

        /* At this point we have:
         *
         * P:
         *   L:               <--- self
         *     LL: ...
         *
         *     C:             <--- repl
         *       LR: ...
         *       R:  ...
         */

        self.update();
    }

    fn rotate_left(&mut self) {
        /* Let's consider thw following structure:
         *
         * P:
         *   C:               <--- self
         *     L: ...
         *
         *     R:             <--- Some
         *       RL: ...
         *       RR: ...
         */

        let mut repl = self.right.take().unwrap();

        /* At this point we have:
         *
         * P:
         *   C:               <--- self
         *     L: ...
         *
         * And a detached:
         *
         * R:                 <--- repl
         *   RL: ...
         *   RR: ...
         */

        self.right = repl.left.take();

        /* At this point we have:
         *
         * P:
         *   C:               <--- self
         *     L:  ...
         *     RL: ...
         *
         * And a detached:
         *
         * R:                 <--- repl
         *   RR: ...
         */

        std::mem::swap(self, &mut *repl);

        /* At this point we have:
         *
         * P:
         *   R:               <--- self
         *     RR: ...
         *
         * And a detached:
         *
         * C:                 <--- repl
         *   L:  ...
         *   RL: ...
         */

        repl.update();
        self.left = Some(repl);

        /* At this point we have:
         *
         * P:
         *   R:               <--- self
         *     C:             <--- repl
         *       L:  ...
         *       RL: ...
         *
         *     RR: ...
         */

        self.update();
    }

    fn balance(&mut self) {
        self.update();

        match self.factor() {
            -2 => {
                if self.l_factor() > 0 {
                    self.left.as_mut().unwrap().rotate_left();
                }

                self.rotate_right();
            }

            2 => {
                if self.r_factor() < 0 {
                    self.right.as_mut().unwrap().rotate_right();
                }

                self.rotate_left();
            }

            _ => {}
        }
    }

    fn remove_minimum(&mut self) -> Box<Node<K, V>> {
        let mut to_ret = if self.left.is_none() {
            let mut removed = self.right.take().unwrap();
            std::mem::swap(self, &mut *removed);
            removed
        } else if self.left.as_ref().unwrap().left.is_none()
            && self.left.as_ref().unwrap().right.is_none()
        {
            let to_ret = self.left.take().unwrap();
            self.balance();
            to_ret
        } else {
            let to_ret = self.left.as_mut().unwrap().remove_minimum();
            self.balance();
            to_ret
        };

        // sets size and height to 1 since this is a detached node,
        // this line is here only for consistency, we do not use
        // the height and size fields of detached nodes
        to_ret.update();

        to_ret
    }

    pub fn get<B>(&self, key: &B) -> Option<&V>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        match self.key.borrow().cmp(key) {
            Less => self.right.as_ref()?.get(key),
            Greater => self.left.as_ref()?.get(key),
            Equal => Some(&self.value),
        }
    }

    pub fn get_key_value<B>(&self, key: &B) -> Option<(&K, &V)>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        match self.key.borrow().cmp(key) {
            Less => self.right.as_ref()?.get_key_value(key),
            Greater => self.left.as_ref()?.get_key_value(key),
            Equal => Some((&self.key, &self.value)),
        }
    }

    pub fn contains_key<B>(&self, key: &B) -> bool
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        match self.key.borrow().cmp(key) {
            Less => self
                .right
                .as_ref()
                .map_or(false, |node| node.contains_key(key)),
            Greater => self
                .left
                .as_ref()
                .map_or(false, |node| node.contains_key(key)),
            Equal => true,
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let to_ret = match self.key.cmp(&key) {
            Less => {
                if let Some(right) = self.right.as_mut() {
                    right.insert(key, value)
                } else {
                    self.right = Some(Box::new(Node::new(key, value)));
                    None
                }
            }
            Greater => {
                if let Some(left) = self.left.as_mut() {
                    left.insert(key, value)
                } else {
                    self.left = Some(Box::new(Node::new(key, value)));
                    None
                }
            }
            Equal => Some(std::mem::replace(&mut self.value, value)),
        };

        self.balance();

        to_ret
    }

    pub fn nth_key_value(&self, n: usize) -> Option<(&K, &V)> {
        if self.size < n {
            return None;
        }

        let l_size = self.l_size();
        match l_size.cmp(&n) {
            Equal => Some((&self.key, &self.value)),
            Less => self.right.as_ref()?.nth_key_value(n - l_size - 1),
            Greater => self.left.as_ref().unwrap().nth_key_value(n),
        }
    }

    fn do_remove<B>(&mut self, key: &B) -> Option<Box<Node<K, V>>>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        let mut removed: Option<Box<Node<K, V>>>;
        match self.key.borrow().cmp(key) {
            Less => {
                removed = self.right.as_mut()?.do_remove(key);
                if removed.is_none()
                    && self.right.is_some()
                    && self.right.as_ref().unwrap().size == 0
                {
                    removed = self.right.take();
                }

                self.balance();
            }
            Greater => {
                removed = self.left.as_mut()?.do_remove(key);
                if removed.is_none() && self.left.is_some() && self.left.as_ref().unwrap().size == 0
                {
                    removed = self.left.take();
                }

                self.balance();
            }
            Equal => {
                if self.left.is_none() && self.right.is_none() {
                    self.size = 0;
                    removed = None
                } else if self.right.is_none() {
                    // detach the current node's left
                    let mut repl = self.left.take().unwrap();

                    // replace the current node with the detached left
                    // (the current node becomes detached)
                    std::mem::swap(self, &mut *repl);

                    // sets size and height to 1 since this is a detached node,
                    // this line is here only for consistency, we do not use
                    // the height and size fields of detached nodes
                    repl.update();

                    // removed is self
                    removed = Some(repl)
                } else if self.right.as_ref().unwrap().right.is_none()
                    && self.right.as_ref().unwrap().left.is_none()
                {
                    // attached the current node's left to its right (as left)
                    let left = self.left.take();
                    self.right.as_mut().unwrap().left = left;
                    self.right.as_mut().unwrap().balance();

                    // detach the current node's right
                    let mut repl = self.right.take().unwrap();

                    // replace the current node with the detached right
                    // (the current node becomes detached)
                    std::mem::swap(self, &mut *repl);

                    // sets size and height to 1 since this is a detached node,
                    // this line is here only for consistency, we do not use
                    // the height and size fields of detached nodes
                    repl.update();

                    // removed is the current node
                    removed = Some(repl)
                } else {
                    // detach the minimum in the current node's right subtree
                    let mut repl = self.right.as_mut().unwrap().remove_minimum();

                    // reattach the current node's left and right to the obtained node
                    repl.left = self.left.take();
                    repl.right = self.right.take();
                    repl.balance();

                    // replace the current node with the obtained one
                    // (the current node becomes detached)
                    std::mem::swap(self, &mut *repl);

                    // sets size and height to 1 since this is a detached node,
                    // this line is here only for consistency, we do not use
                    // the height and size fields of detached nodes
                    repl.update();

                    // removed is the current node
                    removed = Some(repl)
                };
            }
        }

        removed
    }

    pub fn remove<B>(&mut self, key: &B) -> Option<V>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        let removed = self.do_remove(key)?;
        Some(removed.value)
    }

    pub fn remove_entry<B>(&mut self, key: &B) -> Option<(K, V)>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        let removed = self.do_remove(key)?;
        Some((removed.key, removed.value))
    }

    fn do_fmt(&self, f: &mut Formatter<'_>, space_count: usize) -> std::fmt::Result
    where
        K: Display,
        V: Display,
    {
        if let Some(right) = &self.right {
            right.do_fmt(f, space_count + 16)?;
        }

        write!(f, "{}", " ".repeat(space_count))?;
        writeln!(
            f,
            "[s: {}, h: {}] {}: {}",
            self.size, self.height, &self.key, &self.value
        )?;

        if let Some(left) = &self.left {
            left.do_fmt(f, space_count + 16)?;
        }

        Ok(())
    }
}

impl<K: Display + Ord, V: Display> Display for Node<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.do_fmt(f, 0)
    }
}
