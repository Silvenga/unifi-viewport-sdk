//! Integration tests for the controller → device adoption flow.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;
use unifi_controller::{adopt_viewport, AdoptionParams};
use unifi_device_viewport::{AdoptionRequest, DeviceServer};

#[tokio::test]
async fn when_controller_adopts_then_device_callback_invoked() {
    let adopted = Arc::new(AtomicBool::new(false));
    let adopted_clone = adopted.clone();

    let server = DeviceServer::new(move |req: AdoptionRequest| {
        assert_eq!(req.username, "ubnt");
        assert_eq!(req.password, "ubnt");
        assert_eq!(req.token, "test-token");
        assert_eq!(req.nvr, "UNVR4");
        assert_eq!(req.controller, "Protect");
        adopted_clone.store(true, Ordering::SeqCst);
        Ok(())
    })
    .unwrap();

    let server_addr = start_server_on_ephemeral_port(server).await;

    let params = AdoptionParams::new("test-token", "console-id", "UNVR")
        .with_hosts(vec!["192.168.0.4:7442".to_owned()])
        .with_nvr("UNVR4");

    let result = adopt_viewport(&server_addr, &params).await.unwrap();
    assert!(result.success);
    assert!(adopted.load(Ordering::SeqCst));
}

#[tokio::test]
async fn when_wrong_password_then_adoption_rejected() {
    let server = DeviceServer::new(|_| Ok(()))
        .unwrap()
        .with_password("custom-password");

    let server_addr = start_server_on_ephemeral_port(server).await;

    let params = AdoptionParams::new("test-token", "console-id", "UNVR");

    let result = adopt_viewport(&server_addr, &params).await;
    assert!(result.is_err());
}

async fn start_server_on_ephemeral_port(server: DeviceServer) -> String {
    let port = pick_free_port().await;
    let server = server.with_port(port);
    tokio::spawn(async move {
        let _ = server.serve().await;
    });

    sleep(Duration::from_millis(100)).await;
    format!("127.0.0.1:{port}")
}

async fn pick_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}
