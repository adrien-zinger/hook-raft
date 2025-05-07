use config::ConfigError;
use serde::{Deserialize, Serialize};

pub type ErrorResult<T> = std::result::Result<T, Box<Error>>;
pub type WarnResult<T> = std::result::Result<T, Box<Warning>>;

macro_rules! throw {
    ($err: expr) => {
        return Err(Box::new($err))
    };
}
pub(crate) use throw;

#[derive(Debug, Deserialize, Serialize)]
pub struct HttpErrorResult {
    pub err_id: String,
    pub message: String,
}

impl std::fmt::Display for HttpErrorResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "network error id: {}\n error message: {}",
            self.err_id, self.message
        ))
    }
}
/// Error that can append in the execution.
///
/// The Errors should be managed by the root caller of the most "mained"
/// function and cause a stop of the node at the end.
#[derive(Debug)]
#[cfg_attr(test, derive(Clone))]
pub enum Error {
    CannotReadSettings(std::sync::Arc<ConfigError>),
    CannotStartRpcServer(String),
    //SerializationFailed(String), todo: serde_json, error handling with '?'
    InitializationFail(&'static str),
    ImpossibleToBootstrap,
    WrongStatus,
}

#[derive(Debug)]
pub enum ServerError {
    CannotDeserializeBody(String),
}

/// Warning that can append in the execution.
///
/// The Errors should be managed by the direct parent function and translated
/// into an `Error` or dismiss based on the context.
///
/// You may want at least print a warning with a `eprintln!()` or something to
/// notify the user that something strange happened.
#[derive(Debug)]
pub enum Warning {
    CommandFail(String),
    Timeout(&'static str),
    BadResult(HttpErrorResult), // When a request return an HttpErrorResult
    WrongResult(&'static str),  // When a request return an unexpected result
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Warning::CommandFail(str) => f.write_str(&format!("warning: command fail, {str}")),
            Warning::Timeout(str) => f.write_str(&format!("warning: timeout, {str}")),
            Warning::BadResult(err) => f.write_str(&format!(
                "warning: bad result:\n{:indent$}",
                err,
                indent = 2
            )),
            Warning::WrongResult(str) => f.write_str(&format!("warning: wrong result, {str}")),
        }
    }
}

impl From<hyper::http::Error> for Box<Warning> {
    fn from(err: hyper::http::Error) -> Self {
        Box::new(Warning::CommandFail(format!(
            "unable to build request:\n{:indent$}",
            err,
            indent = 2
        )))
    }
}

// todo: use '?' with serde_json
//impl From<serde_json::Error> for Box<Error> {
//    fn from(err: serde_json::Error) -> Self {
//        Box::new(Error::SerializationFailed(format!(
//            "serialization failed:\n{:indent$}",
//            err.to_string(),
//            indent = 2
//        )))
//    }
//}

pub mod errors {
    use super::*;
    /***********************************************/
    /*     Const defined errors and warnings       */

    pub const WARN_INFO_TIMEOUT: Warning =
        Warning::Timeout("Timeout on get information client request");
}
