# Viewport Management API

> Status: Endpoint map and schemas cross-validated against decompiled firmware source. Behavior notes from network
> captures between a UNVR controller (Protect 7.1.83 / UniFi OS 5.1.19) and a UP Viewport (firmware `1.4.33`).

## Overview

An HTTP/TLS server runs on the device on port 8080 (hardcoded). The controller connects to this for adoption and
management. All `POST /api/*` endpoints require `username` and `password` in the JSON body.

## TLS

- The device's server generates TLS certificates on factory reset.
- Self-signed certificate, RSA 2048-bit, SHA256withRSA
- Subject / Issuer: `CN=UI RSA, O=UI`.
- Validity: `now - 5 years` to `now + 20 years` (25-year window).
- This certificate is also used as a client certificate for the controller (which the controller stores on adoption).
- No client certificate is required when connecting to the device's API.
- The controller does not verify the device's server certificate (accepts self-signed).
- TLS 1.3 is used and TLS 1.2 is enabled.

> In testing, `ECDSA P-256; CN=localhost` is also accepted by the controller.

## Authentication

### Default Credentials (Pre-Adoption)

Two valid credential pairs:

- `ubnt` / `ubnt`
- `ui` / `ui`

The device publishes TLV type `0x2C` in its discovery response advertising which default credentials are supported
(bit 0 = `ubnt`, bit 1 = `ui`; value `0x03` = both). See [discovery.md](./discovery.md).

### Post-Adoption Credentials

After adoption, when `rest_user` and `rest_pass` are stored, three valid pairs:

- `(storedUser, storedPass)`
- `("ui", storedPass)`
- `("ubnt", storedPass)`

The original `ui`/`ui` and `ubnt`/`ubnt` pairs are replaced. The password is changed via the `changeUserPassword` UCP4
message (see [ucp4.md](./ucp4.md)) or via `device.auth` in `POST /api/settings`.

Note that the `storedUser` does not appear to be used/changed in pratice and that TLV type `0x2C` specifies if `ui` is
supported.

## Endpoint Map

| Path                               | Method | Response                       |
|------------------------------------|--------|--------------------------------|
| `/api/adopt`                       | POST   | `Success` (text/plain)         |
| `/api/info`                        | POST   | JSON: device info              |
| `/api/settings`                    | POST   | Empty body (Content-Length: 0) |
| `/api/support`                     | POST   | ZIP file (~235 KB)             |
| `/api/snapshot`                    | POST   | PNG (3840x2160)                |
| `/api/version`                     | POST   | JSON: apk, codename, etc.      |
| `/api/reboot`                      | POST   | Reboots device                 |
| `/api/reset_image`                 | POST   | Factory reset                  |
| `/api/update_fw`                   | POST   | Triggers OTA broadcast         |
| `/api/set_inform`                  | POST   | Alternative adoption path      |
| `/api/services`                    | POST   | JSON: `{"tv": true}`           |
| `/api/anrtrace`                    | POST   | ZIP of `/data/anr/`            |
| `/api/displayPlaybackMeasurements` | POST   | Toggle preference              |
| `/api/v2/status`                   | GET    | JSON: health stats             |

## Important Endpoints

### `POST /api/adopt`

Adoption request from the controller. Body:

```json
{
  "username": "ubnt",
  "password": "ubnt",
  "hosts": [
    "192.168.0.4:7442"
  ],
  "token": "HiCnv4x4OYoN4wb446C2QbIbPPIWpLOk",
  "protocol": "wss",
  "mode": 0,
  "nvr": "UNVR4",
  "controller": "Protect",
  "consoleId": "53540ea4-b520-512c-af90-ef08f10eb2aa",
  "consoleName": "UNVR"
}
```

| Field         | Type             | Required | Notes                                                  |
|---------------|------------------|----------|--------------------------------------------------------|
| `username`    | string           | Yes      | Auth credential                                        |
| `password`    | string           | Yes      | Auth credential                                        |
| `hosts`       | array of strings | Yes      | Controller WSS endpoints (host:port)                   |
| `token`       | string           | Yes      | 32-char adoption token, used as `x-token` on WebSocket |
| `protocol`    | string           | No       | Default `"wss"`                                        |
| `mode`        | int              | No       | Adoption mode                                          |
| `nvr`         | string           | No       | Default `"UNKNOWN"`                                    |
| `controller`  | string           | No       | Controller type                                        |
| `consoleId`   | string           | No       | Console UUID                                           |
| `consoleName` | string           | No       | Console display name                                   |

Processing:

1. Optionally process `device.auth` if embedded (password change).
2. Reorder `hosts` so the host matching the request's remote IP is first.
3. Store `hosts`, `token`, `nvr`, `protocol`, `consoleId` in persistent state.
4. Start WebSocket client connecting to `wss://{firstHost}:7442`.
5. Return `200 OK` with body `Success` (text/plain).

The controller does not parse the response body - it only checks for the 200 status.

This `hosts` list is stored for future requests (no re-discovery occurs if the controller has an IP change). It appears
that there is some logic to handle non-local network IP's - the `hosts` list is sorted, favoring local network IP's
first.

The `token` is a 32-character random alphanumeric string, generated by the controller when the user clicks "Adopt". The
token is sent to the device in the `POST /api/adopt` body, then the device presents it via the `x-token` header on the
WebSocket connection.

If the device doesn't respond to the adoption, the controller keeps trying (marking the device as offline in the
console) with a new token. Returning a 400 causes the console to raise an error with adoption.

### `POST /api/info`

Returns device info JSON:

```json
{
  "mac": "E4388334091E",
  "type": "UP Viewport",
  "version": "1.4.33",
  "sw_version": "up-viewport-1.4.33",
  "uptime": 2479,
  "network": {
    "linkSpeedMbps": 1000
  }
}
```

Uptime is in seconds.

### `POST /api/settings`

The controller has been observed hitting this route repeatedly when the firmware is set to `1.0.0` (invalid firmware?).
The controller is attempting to enable ADB (I haven't verified if the controller actually connects to ADB afterward).

Body:

```json
{
  "username": "ubnt",
  "password": "ubnt",
  "device": {
    "auth": {
      "username": "...",
      "passwordOld": "...",
      "passwordNew": "..."
    },
    "wifi": {
      "enabled": true,
      "ssid": "...",
      "auth": "psk",
      "key": "..."
    },
    "net": {
      "type": "static|dhcp",
      "ip": "...",
      "mask": "...",
      "dns1": "...",
      "dns2": "...",
      "gw": "..."
    },
    "adb": true,
    "volume": 7
  }
}
```

Response is always `200 OK` with empty body (Content-Length: 0).

`device.auth` (password change):

- Compares `passwordOld` against the stored password.
- If match: stores `username` and `passwordNew`.
- If no match: silently succeeds (200 OK, no change).

`device.adb` - boolean. Enables ADB over TCP 5555. The firmware disables ADB authentication, so the ADB service will
accept any remote connection. This appears to persist across reboots.

`device.volume` - integer. Sets the media stream volume (Android `STREAM_MUSIC`).

`wifi` - I don't really know why Wi-Fi is here... there is a Wi-Fi chip onboard, but is in a DOWN state. I haven't tried
changing this. There's also Bluetooth on the board, also unused.

### `GET /api/v2/status`

Returns health stats JSON (no auth required from localhost):

```json
{
  "cpu0": "1613MHz",
  "cpu6": "826MHz",
  "memUsed": "28%",
  "temp": "44°C",
  "eth0": "/192.168.0.201/24 [/192.168.0.255]",
  "wsClientCreated": 4,
  "pixelRateUsage": 0.0,
  "totalDecoderCount": 0,
  "swDecoder": 0,
  "startTime": "40m 10.245s",
  "playbackStats": []
}
```

`cpu0` is a BIG core, while `cpu6` is a LITTLE core.

### `POST /api/set_inform` (Alternative Adoption)

Takes `inform_url`, extracts the host, builds a `host:7442` array, and starts the WebSocket client. Does not set
`consoleId`.

## Error Handling

- Auth failure → `401 Unauthorized`.
- `JSONException` in a `settings` sub-object → `500 Internal Server Error` with the exception message.
- Unknown route → `404 Not Found`.
- Password mismatch in `device.auth` → `200 OK` (silent success, no change).
- Static IP validation failure → `200 OK` (logged but no error response).
