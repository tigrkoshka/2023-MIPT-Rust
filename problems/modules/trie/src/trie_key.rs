#![forbid(unsafe_code)]

use std::str::Chars;

pub trait ToKeyIter {
    type Item: Clone;
    type KeyIter<'a>: Iterator<Item = Self::Item>
    where
        Self: 'a;

    fn key_iter(&self) -> Self::KeyIter<'_>;
}

impl ToKeyIter for str {
    type Item = char;
    type KeyIter<'a> = Chars<'a>;

    fn key_iter(&self) -> Self::KeyIter<'_> {
        self.chars()
    }
}

impl ToKeyIter for String {
    type Item = char;
    type KeyIter<'a> = Chars<'a>;

    fn key_iter(&self) -> Self::KeyIter<'_> {
        self.chars()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait FromKeyIter {
    type Dst;

    fn to_key(self) -> Self::Dst;
}

impl FromKeyIter for Chars<'_> {
    type Dst = String;

    fn to_key(self) -> Self::Dst {
        self.collect()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait TrieKey<'a>: ToKeyIter
where
    Self: 'a,
    Self::KeyIter<'a>: FromKeyIter<Dst = Self>,
{
}
