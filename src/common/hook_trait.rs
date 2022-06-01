use crate::log_entry::Term;
pub trait Hook: Send + Sync {
    // all script method are fn
    // when declaring a Node, give an struct that impl Hook

    // Node(hooked: Box<dyn Hook>)
    // have a default implemention ? <--
    // (this is consensus stuffs)
    //
    // VS
    //
    // (this is network / user interaction stuffs)
    // hook : <-- default mode in binary
    // - binary is lib + reading script folder
    // - service.systemd reading sockets
    fn update_node(&self) -> bool;
    fn pre_append_term(&self, term: &Term) -> bool;
    fn append_term(&self, term: &Term) -> bool;
    fn commit_term(&self, term: &Term) -> bool;
    fn prepare_term(&self) -> String;
    // TODO
}

// todo, according to the README specification we should implement other methods,
// one for each functions here.

// todo, the user should be able to provide a dir where the scripts are located
// instead of the current_dir

// todo, remove unwrap and consider using a warning, we need to define the behavior
// for each script when we catch a fail.

// todo, that file should be reworked, each function should have a timeout taken from
// the setting file (cannot be staticly compute on start), each should also have a
// guard prevention from panic, instead, return the default value.
