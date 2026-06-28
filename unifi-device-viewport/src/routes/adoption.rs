use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::{info, warn};

const ADOPT_PATH: &str = "/api/adopt";

/// The adoption payload sent by the controller to the device via POST `/api/adopt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionRequest {
    /// The device user's username (e.g. `ui` or `ubnt`).
    pub username: String,

    /// The device user's password (matches the factory default or the
    /// overridden value set on the server).
    pub password: String,

    /// The controller's WebSocket endpoints (e.g. `["192.168.0.4:7442"]`).
    pub hosts: Vec<String>,

    /// The adoption token used to authenticate the WebSocket connection.
    pub token: String,

    /// The WebSocket protocol scheme (`"wss"`).
    pub protocol: String,

    /// Adoption mode (observed: `0`).
    pub mode: u32,

    /// The NVR model string (e.g. `"UNVR4"`).
    pub nvr: String,

    /// The controller application name (e.g. `"Protect"`).
    pub controller: String,

    /// The console's unique ID.
    pub console_id: String,

    /// The console's display name.
    pub console_name: String,
}

/// Callback invoked when a valid adoption request is received.
pub type AdoptionCallback = Arc<dyn Fn(AdoptionRequest) -> Result<(), String> + Send + Sync>;

/// Handles POST `/api/adopt`. Returns the response, whether the route matched,
/// and the raw request body bytes (for logging on unmatched routes).
pub async fn handle_adoption(
    req: Request<Incoming>,
    password: &str,
    callback: &AdoptionCallback,
) -> Result<(Response<Full<Bytes>>, bool, Bytes), Infallible> {
    let matched = req.method() == Method::POST && req.uri().path() == ADOPT_PATH;

    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!(error = %e, "Failed to read request body");
            if matched {
                return Ok((
                    text_response(StatusCode::BAD_REQUEST, "Bad Request"),
                    true,
                    Bytes::new(),
                ));
            } else {
                return Ok((not_found(), false, Bytes::new()));
            }
        }
    };

    if !matched {
        return Ok((not_found(), false, body_bytes));
    }

    let adoption_req: AdoptionRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            warn!(error = %e, "Failed to parse adoption JSON");
            return Ok((
                text_response(StatusCode::BAD_REQUEST, "Bad Request"),
                true,
                body_bytes,
            ));
        }
    };

    if adoption_req.password != password {
        warn!("Adoption rejected: wrong password");
        return Ok((
            text_response(StatusCode::UNAUTHORIZED, "Unauthorized"),
            true,
            body_bytes,
        ));
    }

    match callback(adoption_req) {
        Ok(()) => {
            info!("Adoption accepted");
            Ok((text_response(StatusCode::OK, "Success"), true, body_bytes))
        }
        Err(reason) => {
            warn!(reason = %reason, "Adoption callback rejected");
            Ok((
                text_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
                true,
                body_bytes,
            ))
        }
    }
}

fn text_response(status: StatusCode, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(Full::new(Bytes::from(body.to_owned())))
        .unwrap()
}

fn not_found() -> Response<Full<Bytes>> {
    text_response(StatusCode::NOT_FOUND, "Not Found")
}
