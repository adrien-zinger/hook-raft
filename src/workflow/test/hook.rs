use crate::{state::EStatus, Hook, Term};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct TestHook {
    pub pre_append_terms: Arc<Mutex<VecDeque<usize>>>,
}

impl Hook for TestHook {
    fn update_node(&self) -> bool {
        true
    }

    fn pre_append_term(&self, _term: &Term) -> Option<usize> {
        (&mut *self.pre_append_terms.lock().unwrap()).pop_back()
    }

    fn append_term(&self, _term: &Term) -> bool {
        true
    }

    fn commit_term(&self, _term: &Term) -> bool {
        true
    }

    fn prepare_term(&self) -> String {
        String::default()
    }

    fn retreive_term(&self, index: usize) -> Option<Term> {
        None
    }

    fn retreive_terms(&self, from: usize, to: usize) -> Option<Vec<Term>> {
        None
    }

    fn switch_status(&self, status: EStatus) {}
}
