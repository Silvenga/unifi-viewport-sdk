use crate::cert::generate_self_signed_cert;
use crate::error::DeviceError;
use crate::routes::{
    handle_adoption, handle_info, handle_settings, AdoptionCallback, AdoptionRequest,
};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::version::TLS13;
use rustls::ServerConfig;
use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info, warn};

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_PASSWORD: &str = "ubnt";
const DEFAULT_MANAGEMENT_PASSWORD: &str = "ubnt";

/// Builder for the device-side HTTP server over TLS 1.3.
///
/// Routes are dispatched to individual route handlers. Currently supports:
/// - `POST /api/adopt` — adoption endpoint.
/// - `POST /api/info` — device info query.
/// - `POST /api/settings` — management settings.
///
/// # Defaults
/// - Port: `8080`
/// - Bind address: `0.0.0.0`
/// - Adoption password: `ui` (factory default)
/// - Management password: `ubnt` (factory default)
/// - TLS: self-signed certificate, TLS 1.3 only
pub struct DeviceServer {
    port: u16,
    bind_addr: Ipv4Addr,
    password: String,
    management_password: String,
    firmware: String,
    mac: [u8; 6],
    callback: AdoptionCallback,
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
}

impl DeviceServer {
    /// Creates a new server with the given adoption callback.
    ///
    /// A self-signed TLS certificate is generated immediately on construction.
    pub fn new<F>(callback: F) -> Result<Self, DeviceError>
    where
        F: Fn(AdoptionRequest) -> Result<(), String> + Send + Sync + 'static,
    {
        let (cert, key) = generate_self_signed_cert().map_err(|e| DeviceError::Cert(e.into()))?;

        Ok(Self {
            port: DEFAULT_PORT,
            bind_addr: Ipv4Addr::UNSPECIFIED,
            password: DEFAULT_PASSWORD.to_owned(),
            management_password: DEFAULT_MANAGEMENT_PASSWORD.to_owned(),
            firmware: String::new(),
            mac: [0; 6],
            callback: Arc::new(callback),
            cert,
            key,
        })
    }

    /// Sets the TCP port to listen on (default: `8080`).
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the bind address (default: `0.0.0.0`).
    pub fn with_bind_addr(mut self, addr: Ipv4Addr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// Sets the adoption password (default: `ui`).
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }

    /// Sets the management password (default: `ubnt`).
    pub fn with_management_password(mut self, password: impl Into<String>) -> Self {
        self.management_password = password.into();
        self
    }

    /// Sets the firmware version reported by `/api/info`.
    pub fn with_firmware(mut self, firmware: impl Into<String>) -> Self {
        self.firmware = firmware.into();
        self
    }

    /// Sets the MAC address reported by `/api/info`.
    pub fn with_mac(mut self, mac: [u8; 6]) -> Self {
        self.mac = mac;
        self
    }

    /// Starts the TLS HTTP server and runs until the future is dropped.
    pub async fn serve(self) -> Result<(), DeviceError> {
        let config = ServerConfig::builder_with_protocol_versions(&[&TLS13])
            .with_no_client_auth()
            .with_single_cert(vec![self.cert], self.key)?;

        let acceptor = TlsAcceptor::from(Arc::new(config));
        let bind = SocketAddr::new(self.bind_addr.into(), self.port);
        let listener = TcpListener::bind(bind).await?;

        info!(bind_addr = %bind, "Device server listening (TLS 1.3)");

        let password = Arc::new(self.password);
        let management_password = Arc::new(self.management_password);
        let firmware = Arc::new(self.firmware);
        let mac = Arc::new(self.mac);
        let callback = self.callback;

        loop {
            let (tcp_stream, peer) = listener.accept().await?;
            let acceptor = acceptor.clone();
            let password = password.clone();
            let management_password = management_password.clone();
            let firmware = firmware.clone();
            let mac = mac.clone();
            let callback = callback.clone();

            tokio::spawn(async move {
                debug!(from = %peer, "Incoming TLS connection");
                let tls_stream = match acceptor.accept(tcp_stream).await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!(from = %peer, error = %e, "TLS handshake failed");
                        return;
                    }
                };

                let io = TokioIo::new(tls_stream);

                let service = service_fn(move |req: Request<Incoming>| {
                    let password = password.clone();
                    let management_password = management_password.clone();
                    let firmware = firmware.clone();
                    let mac = mac.clone();
                    let callback = callback.clone();
                    async move {
                        dispatch(
                            req,
                            &password,
                            &management_password,
                            &firmware,
                            &mac,
                            &callback,
                        )
                        .await
                    }
                });

                if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                    warn!(from = %peer, error = %e, "HTTP connection error");
                }
            });
        }
    }
}

async fn dispatch(
    req: Request<Incoming>,
    password: &str,
    management_password: &str,
    firmware: &str,
    mac: &[u8; 6],
    callback: &AdoptionCallback,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let headers = req.headers().clone();

    let Ok((response, handled, body)) = match (req.method().clone(), req.uri().path().to_owned()) {
        (Method::POST, p) if p == "/api/adopt" => handle_adoption(req, password, callback).await,
        (Method::POST, p) if p == "/api/info" => handle_info(req, password, firmware, mac).await,
        (Method::POST, p) if p == "/api/settings" => {
            handle_settings(req, management_password).await
        }
        _ => {
            let body = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => Bytes::new(),
            };
            let body_str = String::from_utf8_lossy(&body);
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap();
            debug!(method = %method, path = %path, status = %response.status(), headers = ?headers, body = %body_str, "No route matched");
            return Ok(response);
        }
    };

    if handled {
        let body_str = String::from_utf8_lossy(&body);
        debug!(method = %method, path = %path, status = %response.status(), headers = ?headers, body = %body_str, "Request handled");
    } else {
        let body_str = String::from_utf8_lossy(&body);
        debug!(method = %method, path = %path, status = %response.status(), headers = ?headers, body = %body_str, "No route matched");
    }

    Ok(response)
}
