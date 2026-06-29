# ViewPort Device Summary

This summary attempts to show the general shape of the ViewPort from a protocol perspective.

## Device Identity

These values identify a ViewPort on the network.

| Field              | Value                                                                                |
|--------------------|--------------------------------------------------------------------------------------|
| Device type string | `UP Viewport`                                                                        |
| Sysid              | `0xe980` (sent in `x-sysid` WebSocket header; `0x80E9` in discovery TLV `0x10`)      |
| Firmware version   | `1.4.33`                                                                             |
| Firmware string    | `UPV.qcs605.v1.4.33.0.4698daf26.260416.1114`                                         |
| GUID               | `1385fe74-06ad-496f-933e-c1785e3d7947` (constant for all ViewPorts on this firmware) |
| Hostname           | `UP Viewport`                                                                        |
| Platform           | `UP Viewport`                                                                        |

## Device Lifecycle

### Boot / Factory Default State

- `is_default = 1` (unadopted).
- No stored controller address, token, or `consoleId`.
- Discovery responses carry `is_default = 0x01` and omit the controller ID TLV (`0x26`).
- HTTP API available with default credentials `ubnt`/`ubnt` and `ui`/`ui`.
- No WebSocket connection to any controller.

### Adoption

Triggered by `POST /api/adopt` from the controller. The ViewPort:

1. Validates the username/password.
2. Stores `hosts`, `token`, `nvr`, `protocol`, `consoleId` in persistent state.
3. Starts a WebSocket client connecting to `wss://{firstHost}:7442`.
4. Returns `200 OK` with the plaintext body `Success`.

After adoption:

- `is_default = 0` (adopted).
- Discovery responses include the controller ID TLV (`0x26`).

### WebSocket Connection

On connect the ViewPort sends the headers listed in [ucp4.md](./ucp4.md) (Adoption Flow). The post-adoption message
sequence (`getInfo`, `getConsoleInfo`, `networkStatus`, `changeUserPassword`, `configure`, `enableUpdatesChannel`,
`getStreamAlias`) is also documented there.

### Disconnection / Retry

- Connect timeout: 5 seconds.
- On `SocketTimeoutException`: call `onDisconnect`, clear slot/camera repositories, retry.
- Retry delay: 5 seconds.

Note, no automatic retry on the updates channel (`retryOnConnectionFailure(false)`) and a main channel disconnect
triggers a full reconnect cycle (re-establishes both the main and updates channels).

### Factory Reset

- Clears all stored state (`hosts`, `token`, `consoleId`, NVR fingerprint, credentials).
- Returns the ViewPort to the factory default state (`is_default = 1`).
- Regenerates the TLS certificate.
- Triggered by the `factoryReset` UCP4 message or `POST /api/reset_image`.
