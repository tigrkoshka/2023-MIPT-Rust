#![forbid(unsafe_code)]

use std::cmp;

pub fn longest_common_prefix(strs: Vec<&str>) -> String {
    let mut strs = strs.iter();
    let first = match strs.next() {
        None => return "".to_owned(),
        Some(s) => s,
    };
    let mut prefix = *first;

    for s in strs {
        let idx = prefix
            .char_indices()
            .zip(s.chars())
            .find_map(|((i, c1), c2)| if c1 != c2 { Some(i) } else { None })
            .unwrap_or(cmp::min(prefix.len(), s.len()));

        prefix = &prefix[..idx];
    }

    prefix.to_owned()
}
