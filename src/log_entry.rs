// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use std::{
    cmp::Ordering,
    collections::{hash_map::Iter, HashMap},
    fmt::Display,
    iter::Map,
    ops::Range,
};

use crate::node::NodeInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LogEntry {
    pub id: usize,
    pub content: String,
}

impl Default for LogEntry {
    fn default() -> Self {
        Self {
            id: 0,
            content: "".into(),
        }
    }
}

impl Default for Entries {
    fn default() -> Self {
        let mut h = HashMap::default();
        h.insert(0, "".to_string());
        Self(h)
    }
}

impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.content == other.content
    }
}
impl PartialOrd for Term {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.id.partial_cmp(&other.id) {
            Some(core::cmp::Ordering::Equal) => Some(Ordering::Equal),
            ord => ord,
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Entries(HashMap<usize, String>);
pub type Term = LogEntry;
impl Term {
    pub fn _new<T: Display>(id: usize, content: T) -> Self {
        Term {
            id,
            content: content.to_string(),
        }
    }
    pub fn parse_conn(&self) -> Option<NodeInfo> {
        if !self.content.starts_with("conn:") {
            return None;
        }
        let mut c = self.content.clone();
        c.drain(.."conn:".len());
        match serde_json::from_str::<NodeInfo>(&c.to_string()) {
            Ok(res) => Some(res),
            _ => None,
        }
    }
}
type F = fn((&usize, &String)) -> LogEntry;
impl Entries {
    /// Same as default but let the map empty
    pub fn new() -> Self {
        Self(HashMap::default())
    }
    pub fn iter(&self) -> Map<Iter<usize, String>, F> {
        self.0.iter().map(|(id, content)| Term {
            id: *id,
            content: content.clone(),
        })
    }
    pub fn contains(&self, term: &LogEntry) -> bool {
        self.0.contains_key(&term.id)
    }
    pub fn insert(&mut self, term: &LogEntry) -> bool {
        if self.contains(term) {
            if let Some(content) = self.0.get_mut(&term.id) {
                if *content != term.content {
                    *content = term.content.clone();
                    let mut to_remove_list = vec![];
                    for (i, _) in self.0.iter() {
                        if *i > term.id {
                            to_remove_list.push(*i)
                        }
                    }
                    for to_remove in to_remove_list {
                        self.0.remove(&to_remove);
                    }
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            self.0.insert(term.id, term.content.clone());
            true
        }
    }
    pub fn append(&mut self, term_content: String) -> Term {
        let t = Term {
            id: self.len(),
            content: term_content,
        };
        self.insert(&t);
        t
    }
    /// Find the term befor the given input
    pub fn previous(&self, term: &Term) -> Option<Term> {
        for i in term.id - 1..0 {
            if let Some(t) = self.find(i) {
                return Some(t);
            }
        }
        None
    }
    pub fn find(&self, index: usize) -> Option<LogEntry> {
        self.0.get(&index).map(|c| LogEntry {
            id: index,
            content: c.clone(),
        })
    }
    pub fn back(&self) -> Option<LogEntry> {
        self.0.get(&(self.0.len() - 1)).map(|c| LogEntry {
            id: self.0.len() - 1,
            content: c.clone(),
        })
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn get_copy_range(&self, range: Range<usize>) -> Option<Entries> {
        // todo: that can be very better, but waiting for a full development of a kind
        //       of memmapped-file for that data structure.
        let entries = self.iter_range(range);
        let mut ret = Entries::new();
        for e in entries {
            match self.find(e.id) {
                Some(_) => ret.insert(&e),
                _ => return None,
            };
        }
        Some(ret)
    }
    pub fn iter_range(&self, range: Range<usize>) -> Vec<LogEntry> {
        let mut ret = vec![];
        for index in range {
            if let Some(t) = self.find(index) {
                ret.push(t)
            }
        }
        ret
    }
}
