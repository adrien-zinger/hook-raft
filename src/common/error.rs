use crate::api::io_msg::HttpErrorResult;
use config::ConfigError;

pub type ErrorResult<T> = std::result::Result<T, Box<Error>>;
pub type WarnResult<T> = std::result::Result<T, Box<Warning>>;

macro_rules! throw {
    ($err: expr) => {
        return Err(Box::new($err))
    };
}
pub(crate) use throw;
// TODO: make a proposal with thiserror

/// Error that can append in the execution.
///
/// The Errors should be managed by the root caller of the most "mained"
/// function and cause a stop of the node at the end.
#[derive(Debug)]
pub enum Error {
    CannotReadSettings(ConfigError),
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
    use crate::api::io_msg::HttpResult;

    /***********************************************/
    /*     Const defined errors and warnings       */

    pub const WARN_INFO_TIMEOUT: Warning =
        Warning::Timeout("Timeout on get information client request");

    /***********************************************/
    /* ERRORS USED BY THE SERVER API              **/
    /***********************************************/

    lazy_static::lazy_static! {
        pub static ref I_DONT_NOW_THE_LEADER: String = {
            serde_json::to_string(&HttpResult::Error(HttpErrorResult {
                err_id: "512".to_string(),
                message: "sorry I don't know the leader of the network".to_string(),
            }))
            .unwrap()
        };

        pub static ref ERR_APPEND_TERM_SERVER_GENERIC: String = {
            serde_json::to_string(&HttpResult::Error(HttpErrorResult {
                err_id: "513".to_string(),
                message: "Server side generic error on append term".to_string(),
            }))
            .unwrap()
        };
    }

    #[cfg(test)]
    #[test]
    fn deser_server_err() {
        let _ = *I_DONT_NOW_THE_LEADER;
        let _ = *ERR_APPEND_TERM_SERVER_GENERIC;
    }
}
