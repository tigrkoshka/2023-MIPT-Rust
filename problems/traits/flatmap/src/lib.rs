#![forbid(unsafe_code)]

use std::{borrow::Borrow, iter::FromIterator, mem::swap, ops::Index, vec::IntoIter};

////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, PartialEq, Eq)]
pub struct FlatMap<K, V>(Vec<(K, V)>);

impl<K: Ord, V> FlatMap<K, V> {
    pub fn new() -> Self {
        FlatMap(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn as_slice(&self) -> &[(K, V)] {
        self.0.as_slice()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.0.binary_search_by(|pair| pair.0.cmp(&key)) {
            Ok(i) => {
                let mut pair = (key, value);
                swap(&mut self.0[i], &mut pair);
                Some(pair.1)
            }
            Err(i) => {
                self.0.insert(i, (key, value));
                None
            }
        }
    }

    pub fn get<B>(&self, key: &B) -> Option<&V>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        match self.0.binary_search_by(|pair| pair.0.borrow().cmp(key)) {
            Ok(i) => Some(&self.0[i].1),
            Err(_) => None,
        }
    }

    pub fn remove<B>(&mut self, key: &B) -> Option<V>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        self.remove_entry(key).map(|(_, value)| value)
    }

    pub fn remove_entry<B>(&mut self, key: &B) -> Option<(K, V)>
    where
        K: Borrow<B>,
        B: Ord + ?Sized,
    {
        match self.0.binary_search_by(|pair| pair.0.borrow().cmp(key)) {
            Ok(i) => Some(self.0.remove(i)),
            Err(_) => None,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<B, K, V> Index<&B> for FlatMap<K, V>
where
    K: Ord + Borrow<B>,
    B: Ord + ?Sized,
{
    type Output = V;

    fn index(&self, index: &B) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<K: Ord, V> Extend<(K, V)> for FlatMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K: Ord, V> From<Vec<(K, V)>> for FlatMap<K, V> {
    fn from(mut vec: Vec<(K, V)>) -> Self {
        vec.sort_by(|first, second| first.0.cmp(&second.0));

        // leave only the last element on duplicate keys
        vec.reverse();
        vec.dedup_by(|first, second| first.0.eq(&second.0));
        vec.reverse();

        FlatMap(vec)
    }
}

impl<K: Ord, V> From<FlatMap<K, V>> for Vec<(K, V)> {
    fn from(flatmap: FlatMap<K, V>) -> Self {
        flatmap.0
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for FlatMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        FlatMap::from(iter.into_iter().collect::<Vec<(K, V)>>())
    }
}

impl<K, V> IntoIterator for FlatMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
