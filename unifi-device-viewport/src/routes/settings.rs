use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tracing::{info, warn};

const SETTINGS_PATH: &str = "/api/settings";

/// The request body for `POST /api/settings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsRequest {
    /// Management username (v1 default: `ubnt`).
    pub username: String,

    /// Management password.
    pub password: String,

    /// Device settings to update.
    pub device: SettingsDevice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsDevice {
    #[serde(default)]
    pub adb: bool,
}

/// Handles `POST /api/settings`. Returns `(response, matched, body_bytes)`.
pub async fn handle_settings(
    req: Request<Incoming>,
    management_password: &str,
) -> Result<(Response<Full<Bytes>>, bool, Bytes), Infallible> {
    let matched = req.method() == Method::POST && req.uri().path() == SETTINGS_PATH;

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

    let settings_req: SettingsRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            warn!(error = %e, "Failed to parse settings JSON");
            return Ok((
                text_response(StatusCode::BAD_REQUEST, "Bad Request"),
                true,
                body_bytes,
            ));
        }
    };

    if settings_req.password != management_password {
        warn!("Settings request rejected: wrong password");
        return Ok((
            text_response(StatusCode::UNAUTHORIZED, "Unauthorized"),
            true,
            body_bytes,
        ));
    }

    if settings_req.device.adb {
        info!("ADB enabled via settings request");
    }

    Ok((text_response(StatusCode::OK, ""), true, body_bytes))
}

fn text_response(status: StatusCode, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(Full::new(Bytes::from(body.to_owned())))
        .unwrap()
}

fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}
