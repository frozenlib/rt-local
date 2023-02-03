use derive_ex::derive_ex;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
#[derive_ex(Default)]
#[default(Self::new())]
pub struct AssertPass {
    p: Arc<Mutex<Vec<&'static str>>>,
    print: bool,
}

impl AssertPass {
    pub fn new() -> Self {
        Self::new_with(false)
    }
    pub fn new_with(print: bool) -> Self {
        Self {
            p: Arc::new(Mutex::new(Vec::new())),
            print,
        }
    }

    pub fn pass(&self, s: &'static str) {
        self.p.lock().unwrap().push(s);
        if self.print {
            println!("{s}");
        }
    }
    pub fn assert(&self, s: &[&'static str]) {
        assert_eq!(&*self.p.lock().unwrap(), s);
    }
    pub fn assert_ex(&self, s: &[&[&'static str]]) {
        let mut i = 0;
        let mut e = HashSet::<&str>::new();
        for a in &*self.p.lock().unwrap() {
            while e.is_empty() {
                if i == s.len() {
                    panic!("expect finish but `{a}`");
                }
                e.extend(s[i]);
                i += 1;
            }
            if e.contains(a) {
                e.remove(a);
            } else if e.len() == 1 {
                panic!("expect `{}` but `{}`", e.iter().next().unwrap(), a);
            } else {
                panic!("expect one of `{e:?}` but `{a}`");
            }
        }
        loop {
            if !e.is_empty() {
                if e.len() == 1 {
                    panic!("expect finish but `{}`", e.iter().next().unwrap());
                } else {
                    panic!("expect finish but one of `{e:?}`");
                }
            }
            if i == s.len() {
                break;
            }
            e.extend(s[i]);
            i += 1;
        }
    }
}
