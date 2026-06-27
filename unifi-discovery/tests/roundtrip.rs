//! Integration tests for the discovery protocol round-trip.

use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::time::sleep;
use unifi_discovery::{DeviceInfo, DiscoveryClient, DiscoveryMessage, DiscoveryResponder};

const TEST_PORT: u16 = 47821;

#[tokio::test]
async fn when_responder_listens_then_client_discovers_device() {
    let mut info = DeviceInfo::new();
    info.set_mac([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    info.set_ip(
        [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        Ipv4Addr::new(127, 0, 0, 1),
    );
    info.set_firmware("test-1.0.0");
    info.set_uptime(Duration::from_secs(42));
    info.set_hostname("TestDevice");
    info.set_platform("TestPlatform");
    info.set_is_default(true);
    info.set_guid("550e8400-e29b-41d4-a716-446655440000");

    let responder = DiscoveryResponder::new(move || DiscoveryMessage::InfoResponse(info.clone()))
        .with_port(TEST_PORT)
        .with_bind_addr(Ipv4Addr::LOCALHOST);

    let responder_handle = tokio::spawn(async move { responder.listen().await });

    sleep(Duration::from_millis(100)).await;

    let client = DiscoveryClient::new()
        .with_bind_addr(Ipv4Addr::LOCALHOST)
        .with_broadcast_addr(Ipv4Addr::LOCALHOST)
        .with_port(TEST_PORT)
        .with_response_timeout(Duration::from_secs(2));

    let devices = client.query().await.unwrap();

    responder_handle.abort();

    assert_eq!(devices.len(), 1);
    let discovered = &devices[0];
    assert_eq!(
        discovered.get_mac().unwrap().unwrap(),
        [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
    );
    assert_eq!(
        discovered.get_ip().unwrap().unwrap(),
        Ipv4Addr::new(127, 0, 0, 1)
    );
    assert_eq!(discovered.get_firmware().unwrap(), "test-1.0.0");
    assert_eq!(
        discovered.get_uptime().unwrap().unwrap(),
        Duration::from_secs(42)
    );
    assert_eq!(discovered.get_hostname().unwrap(), "TestDevice");
    assert_eq!(discovered.get_platform().unwrap(), "TestPlatform");
    assert!(discovered.get_is_default().unwrap().unwrap());
    assert_eq!(
        discovered.get_guid().unwrap(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
}
