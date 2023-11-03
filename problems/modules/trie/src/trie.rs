#![forbid(unsafe_code)]

use crate::trie_key::ToKeyIter;
use std::{
    borrow::Borrow,
    collections::{hash_map::Entry::*, HashMap, VecDeque},
    hash::Hash,
    ops::Index,
};

////////////////////////////////////////////////////////////////////////////////

type TrieChildren<K, It, V> = HashMap<It, (VecDeque<It>, Trie<K, V>)>;

pub struct Trie<K: ToKeyIter, V>
where
    K::Item: Eq + Hash,
{
    value: Option<V>,
    children: TrieChildren<K, K::Item, V>,
    len: usize,
}

impl<K: ToKeyIter, V> Default for Trie<K, V>
where
    K::Item: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K: ToKeyIter, V> Trie<K, V>
where
    K::Item: Eq + Hash,
{
    pub fn new() -> Self {
        Trie {
            value: None,
            children: HashMap::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_none() && self.children.is_empty()
    }

    fn new_from_value(value: V) -> Self {
        Trie {
            value: Some(value),
            children: HashMap::new(),
            len: 0,
        }
    }

    fn lcp<B>(
        key: &mut B::KeyIter<'_>,
        vd: &VecDeque<B::Item>,
    ) -> (VecDeque<B::Item>, Option<B::Item>)
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        if vd.is_empty() {
            return (VecDeque::new(), None);
        }

        let mut lcp = VecDeque::new();
        loop {
            match key.next() {
                None => break (lcp, None),
                Some(next) => {
                    if vd[lcp.len()] == next {
                        lcp.push_back(next);

                        // do not read the next value from iterator
                        // if lcp is the whole prefix
                        if lcp.len() == vd.len() {
                            break (lcp, None);
                        }

                        continue;
                    }

                    break (lcp, Some(next));
                }
            }
        }
    }

    fn do_insert<B>(&mut self, mut key: B::KeyIter<'_>, value: V) -> Option<V>
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let mut curr = self;
        return loop {
            match key.next() {
                None => break curr.value.replace(value),
                Some(next) => {
                    match curr.children.entry(next.clone()) {
                        Occupied(mut entry) => {
                            let (prefix, _) = entry.get();
                            let (lcp, next_val) = Self::lcp(&mut key, prefix);

                            // go to the next node if full match to prefix
                            if lcp.len() == prefix.len() {
                                let (_, mut_child) = entry.into_mut();
                                curr = mut_child;
                                continue;
                            }

                            // have: | curr | -- next + prefix --> | child |
                            // want: | curr | -- next + lcp    --> | new_child |
                            //            | new_child | -- prefix - lcp           --> | child |
                            //            | new_child | -- next_val + key.collect --> | new from value |

                            // prefix - lcp
                            let mut left_old_prefix: VecDeque<_> =
                                prefix.iter().skip(lcp.len()).cloned().collect();

                            // divide (prefix - lcp) to (first_elem, other_elems)
                            let left_old_prefix_first = left_old_prefix.pop_front().unwrap();

                            // | curr | -- next + lcp --> | new_child |
                            let (_, child_removed) = entry.insert((lcp, Trie::new()));

                            let (_, new_child) = entry.get_mut();

                            // | new_child | -- prefix - lcp --> | child |
                            new_child
                                .children
                                .insert(left_old_prefix_first, (left_old_prefix, child_removed));

                            // if next_val is none, then lcp exhausted the iterator,
                            // so the path specified by the initial key leads to the new_child
                            match next_val {
                                None => new_child.value = Some(value),
                                Some(next_val_some) => {
                                    new_child.children.insert(
                                        next_val_some,
                                        (key.collect(), Self::new_from_value(value)),
                                    );
                                }
                            }
                        }
                        Vacant(v) => {
                            v.insert((key.collect(), Self::new_from_value(value)));
                        }
                    }

                    break None;
                }
            }
        };
    }

    pub fn insert<B>(&mut self, key: &B, value: V) -> Option<V>
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let replaced = self.do_insert(key.key_iter(), value);
        if replaced.is_none() {
            self.len += 1;
        }

        replaced
    }

    pub fn get<B>(&self, key: &B) -> Option<&V>
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let mut it = key.key_iter();
        let mut curr = self;
        return loop {
            match it.next() {
                None => break curr.value.as_ref(),
                Some(next) => match curr.children.get(&next) {
                    None => break None,
                    Some((prefix, child)) => {
                        let (lcp, _) = Self::lcp(&mut it, prefix);
                        if lcp.len() == prefix.len() {
                            curr = child;
                            continue;
                        }

                        break None;
                    }
                },
            }
        };
    }

    pub fn get_mut<B>(&mut self, key: &B) -> Option<&mut V>
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let mut it = key.key_iter();
        let mut curr = self;
        return loop {
            match it.next() {
                None => break curr.value.as_mut(),
                Some(next) => match curr.children.get_mut(&next) {
                    None => break None,
                    Some((prefix, child)) => {
                        let (lcp, _) = Self::lcp(&mut it, prefix);
                        if lcp.len() == prefix.len() {
                            curr = child;
                            continue;
                        }

                        break None;
                    }
                },
            }
        };
    }

    pub fn contains<B>(&self, key: &B) -> bool
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        self.get(key).is_some()
    }

    pub fn starts_with<B>(&self, key: &B) -> bool
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let mut it = key.key_iter();
        let mut curr = self;
        return loop {
            match it.next() {
                None => break true,
                Some(next) => match curr.children.get(&next) {
                    None => break false,
                    Some((prefix, child)) => {
                        let (lcp, next_val) = Self::lcp(&mut it, prefix);
                        if lcp.len() == prefix.len() {
                            curr = child;
                            continue;
                        }

                        break next_val.is_none();
                    }
                },
            }
        };
    }

    fn do_remove<B>(&mut self, mut key: B::KeyIter<'_>) -> (Option<V>, bool)
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        match key.next() {
            None => (self.value.take(), self.children.is_empty()),
            Some(next) => match self.children.get_mut(&next) {
                None => (None, false),
                Some((prefix, child)) => {
                    let (lcp, _) = Self::lcp(&mut key, prefix);
                    if lcp.len() != prefix.len() {
                        return (None, false);
                    }

                    let (removed, delete_branch) = child.do_remove(key);
                    if delete_branch {
                        self.children.remove(&next);
                    }

                    (removed, self.children.is_empty() && self.value.is_none())
                }
            },
        }
    }

    pub fn remove<B>(&mut self, key: &B) -> Option<V>
    where
        K: Borrow<B>,
        B: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let (removed, _) = self.do_remove(key.key_iter());
        if removed.is_some() {
            self.len -= 1;
        }

        removed
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<K: ToKeyIter, V, B> Index<&B> for Trie<K, V>
where
    K::Item: Eq + Hash,
    K: Borrow<B>,
    B: ToKeyIter<Item = K::Item> + ?Sized,
{
    type Output = V;

    fn index(&self, index: &B) -> &Self::Output {
        self.get(index).unwrap()
    }
}
