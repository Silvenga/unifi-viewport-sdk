Goal: Implement a RUST SDK that can mimic a ViewPort that can be discovered, adopted, and configured by a Protect
console. For testing the ViewPort device SDK, a minimal controller implementation will be needed.

## ViewPort Implementation

A complete ViewPort implementation must provide:

- [ ] UDP 10001 discovery responder
- [ ] HTTP/TLS server on port 8080
- [ ] Self-signed certificate generation (RSA 2048, `CN=UI RSA, O=UI`)
- [ ] Adoption handler (`POST /api/adopt`)
- [ ] Auth handler (default credentials pre-adoption, stored credentials post-adoption)
- [ ] Settings handler (`POST /api/settings`)
- [ ] UCP4 WebSocket client to controller:7442
- [ ] Binary frame parser / serializer (header part + body part)
- [ ] Message handlers:
    - [ ] `getInfo`
    - [ ] `getConsoleInfo`
    - [ ] `networkStatus`
    - [ ] `changeUserPassword`
    - [ ] `configure`
    - [ ] `enableUpdatesChannel`
    - [ ] `getStreamAlias`
    - [ ] `reboot`
    - [ ] `factoryReset`
    - [ ] `setVolume`
    - [ ] `updateFirmware`
    - [ ] `updateSoftware`
    - [ ] `supportInfo`
    - [ ] `getAllCameraStatus`
- [ ] Updates channel WebSocket client (second WSS to :7442, `updates` subprotocol, no `x-guid`)
- [ ] Stream WebSocket client to :7446
- [ ] Persistent state: adoption config, credentials, NVR fingerprint, `consoleId`
- [ ] Layout rendering (16 grid layouts)
- [ ] Slot cycling (time-based and motion-based)
- [ ] TOFU fingerprint storage for the controller's server cert
- [ ] Disconnect / retry handling (5s timeout, 5s retry, full reconnect on main-channel loss)

# Controller Implementation

A complete controller implementation must provide:

- [ ] UDP 10001 discovery querier (broadcast to `255.255.255.255` and `233.89.188.1`)
- [ ] HTTP client to ViewPort:8080 (TLS, no cert verification)
- [ ] Adoption sender (`POST /api/adopt`)
- [ ] WSS server on port 7442 (accepts ViewPort connections)
- [ ] TLS server with a self-signed cert
- [ ] Accept the ViewPort's client cert (no verification); extract and forward its fingerprint as `x-fingerprint`
- [ ] Binary frame parser / serializer (same format as the ViewPort side)
- [ ] Message handlers: `getConsoleInfo` response, `getStreamAlias` response
- [ ] Message senders: `getInfo`, `networkStatus`, `changeUserPassword`, `configure`, `enableUpdatesChannel`
- [ ] Updates channel server (accept second WSS with `updates` subprotocol; push `update` messages with
  `modelKey="camera"` and `modifiedKeys`)
