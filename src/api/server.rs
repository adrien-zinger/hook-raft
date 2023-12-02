use super::io_msg::{HttpResult, UpdateNodeInput};
use crate::{
    common::error::{errors, Error, ErrorResult, ServerError},
    node::{Node, NodeInfo},
};
use hyper::service::{make_service_fn, service_fn};
use hyper::{body::Bytes, Uri};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use serde::Deserialize;
use std::{convert::Infallible, net::SocketAddr};
use tracing::{error, trace};

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

async fn body_to_bytes(body: Body) -> Result<Bytes, ServerError> {
    match hyper::body::to_bytes(body).await {
        Ok(body) => Ok(body),
        Err(err) => Err(ServerError::CannotDeserializeBody(format!(
            "error while reading the body: {:?}",
            err
        ))),
    }
}

fn deserialize_body<'a, T: Deserialize<'a>>(body_bytes: &'a Bytes) -> Result<T, ServerError> {
    match serde_json::from_slice(body_bytes) {
        Ok(res) => Ok(res),
        Err(err) => Err(ServerError::CannotDeserializeBody(format!(
            "error while deserialization: {}",
            err
        ))),
    }
}

async fn on_receive_update_node(
    node: &Node,
    bytes: &Bytes,
    response: &mut Response<Body>,
    remote: SocketAddr,
) {
    // todo: check in the body if the `node`
    // value is similar to the caller. (should return an error if not)
    trace!("receive update node request");
    let input: UpdateNodeInput = deserialize_body(bytes).unwrap();
    let addr = format!("{}:{}", remote.ip(), input.port);
    let res = node
        .receive_connection_request(NodeInfo {
            hash: input.hash,
            addr,
        })
        .await;
    match res {
        Some(res) => {
            *response.body_mut() = serde_json::to_string(&HttpResult::UpdateNode(res))
                .unwrap()
                .into()
        }
        None => {
            *response.body_mut() = errors::I_DONT_NOW_THE_LEADER.clone().into();
        }
    }
}

async fn on_receive_append_term(node: &Node, bytes: &Bytes, response: &mut Response<Body>) {
    // todo: check in the body if the `node`
    // value is similar to the caller. (should return an error if not)
    let res = node
        .receive_append_term(deserialize_body(bytes).unwrap())
        .await;
    match res {
        Ok(res) => {
            *response.body_mut() = serde_json::to_string(&HttpResult::AppendTerm(res))
                .unwrap()
                .into()
        }
        Err(_) => {
            *response.body_mut() = errors::ERR_APPEND_TERM_SERVER_GENERIC.clone().into();
        }
    }
}

async fn on_receive_request_vote(node: &Node, bytes: &Bytes, response: &mut Response<Body>) {
    // todo: check in the body if the `node`
    // value is similar to the caller. (should return an error if not)
    let res = node
        .receive_request_vote(deserialize_body(bytes).unwrap())
        .await;
    *response.body_mut() = serde_json::to_string(&HttpResult::RequestVote(res))
        .unwrap()
        .into()
}

async fn dispatch_commands(
    body: Body,
    method: &Method,
    uri: &Uri,
    node: &Node,
    remote: SocketAddr,
) -> Result<Response<Body>, ServerError> {
    let mut response = Response::new(Body::empty());
    let bytes = body_to_bytes(body).await.unwrap();
    match (method, uri.path()) {
        (&Method::POST, "/update_node") => {
            on_receive_update_node(node, &bytes, &mut response, remote).await
        }
        (&Method::POST, "/append_term") => {
            on_receive_append_term(node, &bytes, &mut response).await
        }
        (&Method::POST, "/request_vote") => {
            on_receive_request_vote(node, &bytes, &mut response).await
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };
    Ok(response)
}

fn manage_server_error(
    result: Result<Response<Body>, ServerError>,
) -> Result<Response<Body>, &'static str> {
    // Todo, instead of an error, we should create a Response and a Body with
    // an error inside and a description and keep here a debug message adapted
    // to the error we get.
    match result {
        Ok(response) => Ok(response),
        Err(err) => {
            error!("Server error: {:?}", err);
            Err("Server error")
        }
    }
}

// todo, remove all the "unwrap" here and try to make a nice HTTP response
async fn service(
    req: Request<Body>,
    node: Node,
    remote: SocketAddr,
) -> Result<Response<Body>, hyper::Error> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let body = req.into_body();
    let res = dispatch_commands(body, &method, &uri, &node, remote).await;
    Ok(manage_server_error(res).unwrap())
}

#[cfg(not(feature = "mock_api"))]
pub async fn new(node: Node) -> ErrorResult<()> {
    use crate::common::error::throw;
    use hyper::server::conn::AddrStream;

    let full_addr = &format!("{}:{}", node.settings.addr, node.settings.port);
    trace!("Startup server on {}", full_addr);
    let socket_addr = match full_addr.parse() {
        Ok(addr) => addr,
        Err(err) => throw!(Error::CannotStartRpcServer(format!("{:?}", err))),
    };
    let service = make_service_fn(move |conn: &AddrStream| {
        let node_clone = node.clone();
        let remote_addr = conn.remote_addr();
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                service(req, node_clone.clone(), remote_addr)
            }))
        }
    });

    let server = Server::bind(&socket_addr).serve(service);

    let graceful = server.with_graceful_shutdown(shutdown_signal());
    if let Err(e) = graceful.await {
        error!("server error: {}", e);
    }
    Ok(())
}

#[cfg(feature = "mock_api")]
pub async fn new(node: Node) -> ErrorResult<()> {
    Ok(())
}
