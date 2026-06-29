Goal: Implement a RUST SDK that can mimic a ViewPort that can be discovered, adopted, and configured by a Protect
console. For testing the ViewPort device SDK, a minimal controller implementation will be needed.

## ViewPort Implementation

A complete ViewPort implementation must provide:

- [x] UDP 10001 discovery responder
- [x] HTTP/TLS server on port 8080
- [x] Self-signed certificate generation (RSA 2048, `CN=UI RSA, O=UI`)
- [x] Adoption handler (`POST /api/adopt`)
- [ ] Auth handler (default credentials pre-adoption, stored credentials post-adoption)
  Current: single-password check only, no username validation
  Spec: `ubnt`/`ubnt` + `ui`/`ui` pre-adoption; `(storedUser, storedPass)` + `("ui", storedPass)`
    + `("ubnt", storedPass)` post-adoption
- [x] Settings handler (`POST /api/settings`)
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
- [x] Persistent state: adoption config, credentials, NVR fingerprint, `consoleId`
  Current: `DeviceStorage` trait + `InMemoryStorage` persist `is_adopted`, `password`,
  `controller_id`, `controller_name`, `client_cert_der`, `client_key_der`, `guid`,
  `controller_id_binary`
  Missing: `hosts`, `token`, `protocol`, `nvr` from adoption; NVR fingerprint (TOFU)
  Note: GUID is randomly generated; spec says hardcoded `1385fe74-06ad-496f-933e-c1785e3d7947`
- [ ] Layout rendering (16 grid layouts)
- [ ] Slot cycling (time-based and motion-based)
- [ ] TOFU fingerprint storage for the controller's server cert
- [ ] Disconnect / retry handling (5s timeout, 5s retry, full reconnect on main-channel loss)

## Controller Implementation

A complete controller implementation must provide:

- [x] UDP 10001 discovery querier (broadcast to `255.255.255.255` and `233.89.188.1`)
- [x] HTTP client to ViewPort:8080 (TLS, no cert verification)
- [x] Adoption sender (`POST /api/adopt`)
- [ ] WSS server on port 7442 (accepts ViewPort connections)
- [ ] TLS server with a self-signed cert
- [ ] Accept the ViewPort's client cert (no verification); extract and forward its fingerprint as `x-fingerprint`
- [ ] Binary frame parser / serializer (same format as the ViewPort side)
- [ ] Message handlers: `getConsoleInfo` response, `getStreamAlias` response
- [ ] Message senders: `getInfo`, `networkStatus`, `changeUserPassword`, `configure`, `enableUpdatesChannel`
- [ ] Updates channel server (accept second WSS with `updates` subprotocol; push `update` messages with
  `modelKey="camera"` and `modifiedKeys`)

## Bugs

Known bugs found:

- [ ] Auth handler only validates a single password and ignores the username. The spec requires
  accepting `ubnt`/`ubnt` and `ui`/`ui` pre-adoption, `("ui", storedPass)`, and `("ubnt", storedPass)` post-adoption.
- [ ] Discovery responder omits three "Always present" TLVs:
    - [ ] System ID (`0x10`) — should be `0x80E9` for UP Viewport.
    - [ ] Signal (`0x0F`) — constant `0x00011F90`.
    - [ ] Default Credentials (`0x2C`) — bitfield `0x03` (ubnt + ui supported).
- [ ] TLV length is encoded as a single byte (`type(1) + reserved(1) + length_u8(1)`) instead of the spec's
  `type(1) + length_uint16_BE(2)`. Byte-compatible for all observed frames (values < 256 bytes) but semantically
  inconsistent; would break for TLV values > 255 bytes.
- [ ] GUID is randomly generated on each factory-default boot. The spec says it is hardcoded
  (`1385fe74-06ad-496f-933e-c1785e3d7947`) and identical for all ViewPorts on this firmware.
- [ ] Persistent state does not store `hosts`, `token`, `protocol`, or `nvr` from the adoption request. The spec says
  these are stored for future requests (no re-discovery on controller IP change).
- [ ] NVR fingerprint (TOFU) is not stored. The spec stores the controller's server cert fingerprint on first connection
  in `nvr_fingerprint_2` and rejects mismatches.
