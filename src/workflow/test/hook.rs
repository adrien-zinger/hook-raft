use crate::{state::EStatus, Hook, Term};

pub struct TestHook;

impl Hook for TestHook {
    fn update_node(&self) -> bool {
        true
    }

    fn pre_append_term(&self, _term: &Term) -> Option<usize> {
        None
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
