#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::vec::Vec;

// We need to store the keys in the map (for basic functionality) as well as in the node itself
// (to be able to remove the map entry when removing an element from cache due to overflow).
//
// To avoid cloning the key, we store an Rc.
//
// Note that it does not affect the HashMap hashes due to the fact that K methods
// can be called on Rc<K>. Therefore the HashMap usage is invariant of this change to the key type.

struct Node<K, V> {
    key: Rc<K>,
    value: V,
    prev_index: Option<usize>,
    next_index: Option<usize>,
}

pub struct LRUCache<K, V> {
    map: HashMap<Rc<K>, usize>,
    nodes: Vec<Node<K, V>>,
    head_index: Option<usize>,
    tail_index: Option<usize>,
    capacity: usize,
}

impl<K: Hash + Ord, V> LRUCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            panic!("cannot create a cache with 0 capacity")
        }

        LRUCache {
            map: HashMap::new(),
            nodes: Vec::new(),
            head_index: None,
            tail_index: None,
            capacity,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let node_index_opt = self.map.get(key)?;
        let node_index = *node_index_opt;
        self.move_to_head_by_index(node_index);
        Some(&self.nodes[node_index].value)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let node_index_opt = self.map.get(&key);
        if let Some(&node_index) = node_index_opt {
            self.move_to_head_by_index(node_index);
            return Some(std::mem::replace(&mut self.nodes[node_index].value, value));
        }

        let key_rc = Rc::new(key);

        // prepare the new node to be the head
        let new_node = Node {
            key: Rc::clone(&key_rc),
            value,
            prev_index: None,
            next_index: self.head_index,
        };

        let old_head_index_opt = self.head_index;
        let new_node_index: usize;

        if self.nodes.len() >= self.capacity {
            // since there are at least self.capacity elements, there is a tail
            let tail_index = self.tail_index.unwrap();

            // tail.prev is now tail
            if let Some(prev_tail_index) = self.nodes[tail_index].prev_index {
                self.nodes[prev_tail_index].next_index = None;
                self.tail_index = Some(prev_tail_index);
            }

            // replace the tail with the new node
            let old_tail = std::mem::replace(&mut self.nodes[tail_index], new_node);

            // remove the old entry in the map
            self.map.remove(&old_tail.key);

            // save the new node index for map insertion
            new_node_index = tail_index;
        } else {
            // we can add a new node since the current size is less than capacity
            new_node_index = self.nodes.len();

            // push the node
            self.nodes.push(new_node);

            // if there was no tail (i.e. the cache was empty), the new node is the tail
            if self.tail_index.is_none() {
                self.tail_index = Some(new_node_index);
            }
        }

        // record the new node as head
        self.head_index = Some(new_node_index);

        // set the new node as the previous one for head
        if let Some(old_head_index) = old_head_index_opt {
            self.nodes[old_head_index].prev_index = Some(new_node_index);
        }

        // record the new index for the key
        self.map.insert(Rc::clone(&key_rc), new_node_index);

        None
    }

    fn move_to_head_by_index(&mut self, index: usize) {
        if self.head_index.unwrap() == index {
            return;
        }

        // since the current node is not head (see above), it has a previous one
        let prev_index = self.nodes[index].prev_index.unwrap();

        // link the previous node to the next one
        self.nodes[prev_index].next_index = self.nodes[index].next_index;

        if self.tail_index.unwrap() == index {
            // if we are moving the tail, we need to set the new tail
            self.tail_index = Some(prev_index);
        } else {
            // else we need to link the next node to the previous one

            // since the current node is not tail (see above), it has a next one
            let next_index = self.nodes[index].next_index.unwrap();
            self.nodes[next_index].prev_index = self.nodes[index].prev_index;
        }

        // make the current node head
        self.nodes[index].prev_index = None;
        self.nodes[index].next_index = self.head_index;

        // set the old head's next node to the current node
        self.nodes[self.head_index.unwrap()].prev_index = Some(index);

        // record the current node as head
        self.head_index = Some(index);
    }
}
