use super::{
    io_msg::{
        AppendTermInput, AppendTermResult, RequestVoteInput, RequestVoteResult, UpdateNodeInput,
        UpdateNodeResult,
    },
    Url,
};
use crate::{
    api::io_msg::HttpResult,
    common::{
        config::Settings,
        error::{errors, throw, WarnResult, Warning},
    },
};
use hyper::{Body, Client, Method, Request, Response};
use serde::Serialize;
use std::time::Duration;
use tracing::trace;

async fn run_request(req: Request<Body>, timeout: Duration) -> WarnResult<Response<Body>> {
    let client = Client::new();
    match tokio::time::timeout(timeout, client.request(req)).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => throw!(Warning::CommandFail(format!(
            "warn, client request\n{:indent$}",
            err,
            indent = 2
        ))),
        _ => throw!(errors::WARN_INFO_TIMEOUT),
    }
}

async fn build<T: Serialize>(
    not_serialized_body: T,
    target_uri: String,
    timeout: Duration,
) -> WarnResult<HttpResult> {
    trace!("command: {}", target_uri);
    let req = Request::builder()
        .method(Method::POST)
        .uri(target_uri)
        .body(Body::from(
            serde_json::to_string(&not_serialized_body).unwrap(),
        ))?; // TODO could be simplified
    let mut resp = run_request(req, timeout).await?;
    let body_resp = resp.body_mut();
    if let Ok(resp_bytes) = hyper::body::to_bytes(body_resp).await {
        match serde_json::from_slice(&resp_bytes) {
            Ok(http_result) => Ok(http_result),
            Err(err) => {
                throw!(Warning::CommandFail(format!(
                    "parse HTTP response failed with err:\n{:indent$}",
                    err,
                    indent = 2
                )))
            }
        }
    } else {
        throw!(Warning::CommandFail(
            "hyper failed to read body bytes".to_string(),
        ))
    }
}

/// Try to connect to the distant node at `url` address, return a `Warning`
/// if the connection failed or if it don't success after the duration
/// `settings.response_timeout` in millisecond.
///
/// If success return an `InfoResult`
///
/// Note: The warning should be managed by the direct parent function and
/// translated as an `Error` if needed
pub(crate) async fn post_update_node(
    target: &Url,
    settings: &Settings,
    uuid: [u8; 16],
) -> WarnResult<UpdateNodeResult> {
    let body = UpdateNodeInput {
        hash: uuid,
        port: settings.port.clone(),
    };
    let target_uri = format!("http://{}/update_node", target);
    match build(
        body,
        target_uri,
        Duration::from_millis(settings.response_timeout as u64),
    )
    .await
    {
        Ok(HttpResult::UpdateNode(result)) => Ok(result),
        Ok(HttpResult::Error(err_result)) => {
            throw!(Warning::BadResult(err_result))
        }
        Err(warn) => throw!(*warn),
        _ => throw!(Warning::WrongResult(
            "unexpected result on received 'update_node' response",
        )),
    }
}

/// Send a term to a target
/// If success return an `AppendTermResult`
///
/// Note: The warning should be managed by the direct parent function and
/// translated as an `Error` if needed
pub(crate) async fn post_append_term(
    target: &Url,
    settings: &Settings,
    input: AppendTermInput,
) -> WarnResult<AppendTermResult> {
    let target_uri = format!("http://{}/append_term", target);
    trace!("post term {:?}", input);
    match build(
        input,
        target_uri,
        Duration::from_millis(settings.response_timeout as u64),
    )
    .await
    {
        Ok(HttpResult::AppendTerm(result)) => Ok(result),
        Ok(HttpResult::Error(err_result)) => {
            throw!(Warning::BadResult(err_result))
        }
        Err(warn) => throw!(*warn),
        _ => throw!(Warning::WrongResult(
            "unexpected result on received 'append_term' response",
        )),
    }
}

/// Send a vote request
///
/// Note: The warning should be managed by the direct parent function and
/// translated as an `Error` if needed
pub(crate) async fn post_request_vote(
    target: &Url,
    settings: &Settings,
    input: RequestVoteInput,
) -> WarnResult<RequestVoteResult> {
    let target_uri = format!("http://{}/request_vote", target);
    trace!("request vote to {}", target);
    match build(
        input,
        target_uri,
        Duration::from_millis(settings.response_timeout as u64),
    )
    .await
    {
        Ok(HttpResult::RequestVote(result)) => Ok(result),
        Ok(HttpResult::Error(err_result)) => {
            throw!(Warning::BadResult(err_result))
        }
        Err(warn) => throw!(*warn),
        _ => throw!(Warning::WrongResult(
            "unexpected result on received 'request_vote' response",
        )),
    }
}
