use crate::{log_entry::Term, state::EStatus};
pub trait Hook: Send + Sync {
    // all script method are fn
    // when declaring a Node, give an struct that impl Hook

    // Node(hooked: Box<dyn Hook>)
    // have a default implementation ? <--
    // (this is consensus stuffs)
    //
    // VS
    //
    // (this is network / user interaction stuffs)
    // hook : <-- default mode in binary
    // - binary is lib + reading script folder
    // - service.systemd reading sockets
    fn update_node(&self) -> bool;
    fn pre_append_term(&self, term: &Term) -> Option<usize>;
    fn append_term(&self, term: &Term) -> bool;
    fn commit_term(&self, term: &Term) -> bool;
    fn prepare_term(&self) -> String;
    fn retreive_term(&self, index: usize) -> Option<Term>;
    fn retreive_terms(&self, from: usize, to: usize) -> Option<Vec<Term>>;
    fn switch_status(&self, status: EStatus);
}
