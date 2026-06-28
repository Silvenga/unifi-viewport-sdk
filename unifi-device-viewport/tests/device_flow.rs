//! Integration tests for the ViewPortDevice: discovery + adoption flow.

use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;
use unifi_controller::{adopt_viewport, AdoptionParams};
use unifi_device_viewport::{InMemoryStorage, ViewPortDevice};
use unifi_discovery::DiscoveryClient;

const TEST_MAC: [u8; 6] = [0xFC, 0x34, 0x97, 0xA0, 0xCF, 0xEC];

#[tokio::test]
async fn when_device_listens_then_responds_to_discovery_and_accepts_adoption() {
    let discovery_port = pick_free_port().await;
    let adoption_port = pick_free_port().await;

    let storage = InMemoryStorage::new();

    let device = ViewPortDevice::builder()
        .mac(TEST_MAC)
        .ip(Ipv4Addr::new(127, 0, 0, 1))
        .hostname("UP Viewport")
        .storage(storage)
        .discovery_port(discovery_port)
        .adoption_port(adoption_port)
        .bind_addr(Ipv4Addr::new(127, 0, 0, 1))
        .build()
        .unwrap();

    tokio::spawn(async move {
        let _ = device.listen().await;
    });

    sleep(Duration::from_millis(200)).await;

    let devices = query_discovery(discovery_port).await;
    assert_eq!(devices.len(), 1);
    assert!(devices[0].get_is_default().unwrap().unwrap());

    let params = AdoptionParams::new("test-token", "console-id-1234", "UNVR")
        .with_hosts(vec!["192.168.0.4:7442".to_owned()])
        .with_nvr("UNVR4");

    let result = adopt_viewport(&format!("127.0.0.1:{adoption_port}"), &params)
        .await
        .unwrap();
    assert!(result.success);

    sleep(Duration::from_millis(100)).await;

    let devices = query_discovery(discovery_port).await;
    assert_eq!(devices.len(), 1);
    assert!(!devices[0].get_is_default().unwrap().unwrap());
}

async fn query_discovery(port: u16) -> Vec<unifi_discovery::DeviceInfo> {
    let client = DiscoveryClient::new()
        .with_bind_addr(Ipv4Addr::LOCALHOST)
        .with_broadcast_addr(Ipv4Addr::LOCALHOST)
        .with_port(port)
        .with_response_timeout(Duration::from_secs(2));

    client.query().await.unwrap()
}

async fn pick_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}
