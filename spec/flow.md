# Adoption Flow

```mermaid
sequenceDiagram
    participant Device as ViewPort
    participant DS as NVR (ds proxy :7442)
    participant Backend as NVR (unifi-protect :7448)
    participant NVR8080 as NVR (:8080)

    Note over Device: Device boots, enters factory-default state

    rect rgb(240, 248, 255)
        Note over Device,NVR8080: Discovery & Adoption Token Push
        Device->>DS: UDP 10001 discovery broadcast<br/>(TLV: MAC, IP, firmware, type, sysid, is_default=0x01)
        Note right of Device: Repeats every ~10s
        NVR8080->>Device: TCP :8080 (TLS) — NVR initiates connection
        NVR8080->>Device: Push adoption info (TLS encrypted, not decoded)
        Note over Device: Device receives NVR address + adoption token
    end

    rect rgb(255, 250, 240)
        Note over Device,Backend: WebSocket Connection (ucp4)
        Device->>DS: WSS :7442/ (TLS, client cert)<br/>Sec-WebSocket-Protocol: ucp4<br/>x-ident, x-type, x-sysid, x-mode, x-token, x-adopted: false
        DS->>Backend: WS :7448/ws (plaintext)<br/>Forwards all headers + x-fingerprint
        Note over Backend: verifyUcpClient: validates token,<br/>pins cert fingerprint, sets isAdopted=true
    end

    rect rgb(248, 255, 248)
        Note over Device,Backend: Post-Adoption Message Sequence (ucp4)
        Backend->>Device: getInfo (request, empty body)
        Device->>Backend: getConsoleInfo (request, empty body)
        Device->>Backend: getInfo response<br/>{mac, type, version, sw_version, uptime, network}
        Backend->>Device: getConsoleInfo response<br/>{consoleId, consoleName}
        Backend->>Device: networkStatus (request)
        Device->>Backend: networkStatus response<br/>{linkSpeedMbps}
        Backend->>Device: changeUserPassword<br/>{username, passwordOld, passwordNew}
        Device->>Backend: changeUserPassword response (empty)
        Backend->>Device: configure<br/>{name, nvr, streamProtocol, streamPort, liveview, cameras[]}
        Device->>Backend: configure response (empty)
        Backend->>Device: enableUpdatesChannel<br/>{uri: "wss://NVR:7442", lastUpdateId}
        Device->>Backend: enableUpdatesChannel response (empty)
    end

    rect rgb(255, 248, 255)
        Note over Device,Backend: Stream Alias Requests (ucp4)
        loop For each camera in liveview
            Device->>Backend: getStreamAlias<br/>{camera, channel, type: "ubv"}
            Backend->>Device: getStreamAlias response<br/>{alias, url, rtspUrl}
        end
    end

    rect rgb(240, 255, 240)
        Note over Device,DS: Updates Channel (second WebSocket)
        Device->>DS: WSS :7442/?lastUpdateId=...<br/>Sec-WebSocket-Protocol: updates<br/>Same x-ident, x-type, x-mode headers
        DS->>Backend: WS :7448/ws (plaintext)
        Note over Device,Backend: Push notifications for device state changes<br/>(update messages with modifiedKeys)
    end

    rect rgb(255, 240, 240)
        Note over Device: Stream Pull (port 7446)
        Device->>DS: WSS :7446/{alias}?type=ubv
        DS-->>Device: Camera livestream data
    end

    Note over Device: Discovery continues (is_default=0x00,<br/>includes NVR hardware ID TLV)
```
