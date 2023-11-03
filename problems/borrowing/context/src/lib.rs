#![forbid(unsafe_code)]

use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Default)]
pub struct Context {
    mapping: HashMap<String, Box<dyn Any>>,
    singletons: HashMap<TypeId, Box<dyn Any>>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: Any>(&mut self, key: impl AsRef<str> + 'static, obj: T) {
        let _ = self
            .mapping
            .insert(String::from(key.as_ref()), Box::new(obj));
    }

    pub fn get<T: Any>(&self, key: impl AsRef<str>) -> &T {
        match self.mapping.get(&String::from(key.as_ref())) {
            None => panic!("no such key stored in the context"),
            Some(value) => value.downcast_ref::<T>().unwrap(),
        }
    }

    pub fn insert_singletone<T: Any>(&mut self, obj: T) {
        let _ = self.singletons.insert(TypeId::of::<T>(), Box::new(obj));
    }

    pub fn get_singletone<T: Any>(&self) -> &T {
        match self.singletons.get(&TypeId::of::<T>()) {
            None => panic!("no singleton of such type stored in the context"),
            Some(value) => value.downcast_ref::<T>().unwrap(),
        }
    }
}
