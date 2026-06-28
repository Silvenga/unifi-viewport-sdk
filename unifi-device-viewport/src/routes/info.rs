use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tracing::warn;

const INFO_PATH: &str = "/api/info";

/// The request body for `POST /api/info` — same auth shape as adoption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfoRequest {
    /// The device user's username.
    pub username: String,

    /// The device user's password.
    pub password: String,
}

/// The response body for `POST /api/info` on UCP4 devices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfoResponse {
    /// The device firmware version.
    pub version: String,

    /// The device MAC address (hex, no separators, uppercase).
    pub mac: String,
}

/// Handles `POST /api/info`. Returns `(response, matched, body_bytes)`.
pub async fn handle_info(
    req: Request<Incoming>,
    password: &str,
    device_version: &str,
    device_mac: &[u8; 6],
) -> Result<(Response<Full<Bytes>>, bool, Bytes), Infallible> {
    let matched = req.method() == Method::POST && req.uri().path() == INFO_PATH;

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

    let info_req: InfoRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            warn!(error = %e, "Failed to parse info JSON");
            return Ok((
                text_response(StatusCode::BAD_REQUEST, "Bad Request"),
                true,
                body_bytes,
            ));
        }
    };

    if info_req.password != password {
        warn!("Info request rejected: wrong password");
        return Ok((
            text_response(StatusCode::UNAUTHORIZED, "Unauthorized"),
            true,
            body_bytes,
        ));
    }

    let mac = format!(
        "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        device_mac[0], device_mac[1], device_mac[2], device_mac[3], device_mac[4], device_mac[5]
    );

    let resp = InfoResponse {
        version: device_version.to_owned(),
        mac,
    };

    let body =
        serde_json::to_string(&resp).unwrap_or_else(|_| r#"{"version":"","mac":""}"#.to_owned());

    Ok((text_response(StatusCode::OK, &body), true, body_bytes))
}

fn text_response(status: StatusCode, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body.to_owned())))
        .unwrap()
}

fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}
