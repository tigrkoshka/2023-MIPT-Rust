#![forbid(unsafe_code)]

use crate::node::Node;
use std::borrow::Borrow;
use std::fmt::{Display, Formatter};

pub struct AVLTreeMap<K, V> {
    root: Option<Box<Node<K, V>>>,
}

impl<K: Ord, V> Default for AVLTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord, V> AVLTreeMap<K, V> {
    pub fn new() -> Self {
        AVLTreeMap { root: None }
    }

    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.size())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get<B>(&self, key: &B) -> Option<&V>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        self.root.as_ref()?.get(key)
    }

    pub fn get_key_value<B>(&self, key: &B) -> Option<(&K, &V)>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        self.root.as_ref()?.get_key_value(key)
    }

    pub fn contains_key<B>(&self, key: &B) -> bool
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        if let Some(root) = self.root.as_ref() {
            root.contains_key(key)
        } else {
            false
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(root) = self.root.as_mut() {
            root.insert(key, value)
        } else {
            self.root = Some(Box::new(Node::new(key, value)));
            None
        }
    }

    pub fn nth_key_value(&self, n: usize) -> Option<(&K, &V)> {
        self.root.as_ref()?.nth_key_value(n)
    }

    pub fn remove<B>(&mut self, key: &B) -> Option<V>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        if let Some(root) = self.root.as_mut() {
            let to_ret = root.remove(key);
            if self.is_empty() {
                let root = self.root.take().unwrap();
                return Some(root.value);
            }

            to_ret
        } else {
            None
        }
    }

    pub fn remove_entry<B>(&mut self, key: &B) -> Option<(K, V)>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        if let Some(root) = self.root.as_mut() {
            let to_ret = root.remove_entry(key);
            if self.is_empty() {
                let root = self.root.take().unwrap();
                return Some((root.key, root.value));
            }

            to_ret
        } else {
            None
        }
    }
}

impl<K: Display + Ord, V: Display> Display for AVLTreeMap<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.root {
            None => write!(f, "empty"),
            Some(root) => write!(f, "{}", root),
        }
    }
}
