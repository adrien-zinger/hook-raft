// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use std::{cmp::Ordering, collections::HashMap, fmt::Display};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct LogEntry {
    pub id: usize,
    pub timestamp: String,
    pub content: String,
}

impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.content == other.content && self.timestamp == other.timestamp
    }
}

impl PartialOrd for Term {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.id.partial_cmp(&other.id) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.timestamp.partial_cmp(&other.timestamp) {
            Some(core::cmp::Ordering::Equal) => Some(Ordering::Equal),
            ord => ord,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Entries {
    inner: HashMap<usize, (String, String)>,
    latest: usize,
    commit_index: usize,
    current: LogEntry,
}

pub type Term = LogEntry;

impl Term {
    pub fn _new<T: Display>(id: usize, content: T) -> Self {
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, false);
        Term {
            id,
            timestamp,
            content: content.to_string(),
        }
    }
}

impl Entries {
    /// Same as default
    pub fn new() -> Self {
        Self {
            inner: HashMap::default(),
            latest: 0,
            commit_index: 0,
            current: LogEntry::default(),
        }
    }

    pub fn contains(&self, term: &Term) -> bool {
        self.inner.contains_key(&term.id)
    }

    /// Insert or replace a term. Used in the leader and append_term workflow.
    /// If the term exists, we would like to rollback to avoid conflicts.
    ///
    /// The call of the hook is deferred to the caller if that methods return true.
    pub fn insert(&mut self, term: &Term) {
        if term.id <= self.commit_index {
            panic!("Trying to re-insert an already committed term")
        }
        if term.id <= self.latest {
            for i in term.id..=self.latest {
                self.inner.remove(&i);
            }
        }
        self.latest = term.id;
        self.current = term.clone();
        self.inner
            .insert(term.id, (term.timestamp.clone(), term.content.clone()));
    }

    /// Create a new term from a content
    /// Return the created term
    pub fn append(&mut self, content: String) -> Term {
        let id = self.latest + 1;
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, false);
        debug!("log entry: append term {} {} {}", id, content, timestamp);
        let t = Term {
            id,
            timestamp,
            content,
        };
        self.latest = id;
        self.current = t.clone();
        self.insert(&t);
        t
    }

    pub fn find(&self, index: usize) -> Option<Term> {
        self.inner.get(&index).map(|(t, c)| LogEntry {
            id: index,
            timestamp: t.clone(),
            content: c.clone(),
        })
    }

    pub fn check_commit(&self, index: usize) -> bool {
        if index <= self.commit_index {
            warn!("Trying to re-commit index {index}");
            return false;
        }
        if index > self.latest {
            warn!("Trying to commit an unknown index");
            return false;
        }
        true
    }

    pub fn set_commit(&mut self, index: usize) {
        if self.check_commit(index) {
            self.inner.remove(&index);
            self.commit_index = index;
        }
    }

    /// Find the latest log entry. Creates a new one if
    /// empty.
    pub fn latest(&mut self) -> (bool, Term) {
        if let Some((t, c)) = self.inner.get(&self.latest) {
            debug!("log entry: latest found {}", self.latest);
            (
                false,
                LogEntry {
                    id: self.latest,
                    timestamp: t.clone(),
                    content: c.clone(),
                },
            )
        } else {
            debug!("log entry: latest: append a new empty entry");
            (true, self.append("default".into()))
        }
    }

    pub fn last_index(&self) -> usize {
        self.latest
    }

    pub fn set_last_index(&mut self, index: usize) {
        self.latest = index;
    }

    pub fn commit_index(&self) -> usize {
        self.commit_index
    }

    pub fn current_term(&self) -> Term {
        self.current.clone()
    }
}
