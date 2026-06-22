# UCP4 WebSocket Protocol - Viewer Device-Side Specification

> This spec documents the UCP4 (Ubiquiti Control Protocol v4) WebSocket protocol as observed in network captures between
> a real UP Viewport (firmware `v1.4.33`) and a UNVR running Protect 7.1.83 / UniFi OS 5.1.19.

## Overview

The UP ViewPort uses UCP4, a JSON-over-binary-WebSocket protocol. The ViewPort opens two separate WebSocket connections
to the NVR:

1. **Main control channel** - `Sec-WebSocket-Protocol: ucp4`
2. **Push updates channel** - `Sec-WebSocket-Protocol: updates`

| Aspect              | Value                      |
|---------------------|----------------------------|
| WebSocket port      | `7442`                     |
| WebSocket path      | `/`                        |
| Main subprotocol    | `ucp4`                     |
| Updates subprotocol | `updates`                  |
| Frame type          | Binary (opcode 2)          |
| User-Agent          | `okhttp/4.12.0`            |
| Device type string  | `UP Viewport` (with space) |
| Sysid               | `0xe980`                   |

## Architecture

The NVR runs a Rust proxy process (`ds`) on port 7442 that terminates TLS and forwards plaintext WebSocket traffic to
the Protect Node.js application on `127.0.0.1:7448/ws`. The `ds` proxy extracts the client certificate fingerprint and
adds it as the `x-fingerprint` HTTP header when forwarding to the backend.

```
ViewPort TLS -> ds (port 7442) plaintext -> unifi-protect (port 7448)
```

## Adoption Flow

### Step 1: Device Discovery (UDP 10001)

The device announces itself on the network via UDP 10001, so it appears in the Protect pending-adoption list.

See [discovery](./discovery.md).

### Step 2: NVR Pushes Adoption Info to Device (Port 8080)

From the TLS capture, the NVR initiates a TCP connection to the device on port 8080 (TLS) before the device connects to
the NVR on port 7442.

### Step 3: User Adopts Device

The user adopts the device through the Protect UI. The adoption token reaches the device (likely via the port 8080 push
in Step 2), and the device presents it on the WebSocket connection via the `x-token` header.

### Step 4: WebSocket Connection

The device connects to the NVR on port 7442. The following headers were observed in the captured plaintext traffic
(forwarded by the `ds` proxy to the backend):

| Header                     | Observed Value                         | Notes                                                                                                                                |
|----------------------------|----------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------|
| `Sec-WebSocket-Protocol`   | `ucp4`                                 | Main control channel. `updates` for the push channel.                                                                                |
| `x-ident`                  | `E4388334091E`                         | Device MAC address (no separators)                                                                                                   |
| `x-mode`                   | `0`                                    |                                                                                                                                      |
| `x-type`                   | `UP Viewport`                          | Device type string                                                                                                                   |
| `x-sysid`                  | `0xe980`                               | Observed in UNVR logs; not in the plaintext capture                                                                                  |
| `x-token`                  | `HiCnv4x4OYoN4wb446C2QbIbPPIWpLOk`     | Adoption token. Observed on first connection with `x-adopted: false`. Not observed on subsequent connections with `x-adopted: true`. |
| `x-version`                | `v1.4.33`                              | Firmware version (with `v` prefix)                                                                                                   |
| `x-device-id`              | `7f9c90a2-8152-5d63-214b-d96d6d894b1f` | UUID                                                                                                                                 |
| `x-guid`                   | `1385fe74-06ad-496f-933e-c1785e3d7947` | UUID                                                                                                                                 |
| `x-ip`                     | `192.168.0.201`                        | Device's IP address                                                                                                                  |
| `x-adopted`                | `false` or `true`                      | `false` on first connection, `true` after adoption                                                                                   |
| `user-agent`               | `okhttp/4.12.0`                        | Android OkHttp WebSocket client                                                                                                      |
| `accept-encoding`          | `gzip`                                 |                                                                                                                                      |
| `sec-websocket-extensions` | `permessage-deflate`                   | WebSocket compression                                                                                                                |
| `x-fingerprint`            | `AA:4C:53:FC:...`                      | Added by the `ds` proxy - TLS client cert fingerprint (colon-separated hex, consistent with SHA1 format)                             |
| `x-connection-host`        | `192.168.0.4:7442`                     | Added by the `ds` proxy - the original connection host                                                                               |

### Step 5: Post-Adoption Message Sequence

The following sequence was captured from the plaintext traffic. Timestamps are relative to the WebSocket connection
establishment:

| Time    | Direction | Action                          | Description                                                 |
|---------|-----------|---------------------------------|-------------------------------------------------------------|
| +0ms    | C -> D     | `getInfo`                       | Controller requests device info                             |
| +0ms    | D -> C     | `getConsoleInfo`                | Device requests console info (sent nearly simultaneously)   |
| +207ms  | D -> C     | `getInfo` response              | Device sends its capabilities                               |
| +312ms  | C -> D     | `getConsoleInfo` response       | Controller sends console ID + name                          |
| +619ms  | C -> D     | `networkStatus`                 | Controller requests network status                          |
| +628ms  | D -> C     | `networkStatus` response        | Device sends link speed                                     |
| +628ms  | C -> D     | `changeUserPassword`            | Controller pushes password change                           |
| +636ms  | D -> C     | `changeUserPassword` response   | Device acknowledges                                         |
| +684ms  | C -> D     | `configure`                     | Controller pushes liveview + camera list                    |
| +684ms  | C -> D     | `enableUpdatesChannel`          | Controller tells device to open updates channel             |
| +884ms  | D -> C     | `enableUpdatesChannel` response | Device acknowledges                                         |
| +886ms  | D -> C     | log                             | Device logs `onConfigure: count=16`                         |
| +858ms  | D -> C     | `getStreamAlias` ×N             | Device requests stream aliases (one per camera in liveview) |
| +858ms+ | C -> D     | `getStreamAlias` responses      | Controller returns `{alias, url, rtspUrl}` per camera       |

After this sequence, the device opens a second WebSocket with `sec-websocket-protocol: updates` using the `uri` from
`enableUpdatesChannel`.

## Binary Framing Format

UCP4 uses **binary WebSocket frames** (opcode 2). Each WebSocket frame contains UCP4 messages. Each UCP4 message
consists of a **header part** followed by a **body part**, concatenated in the same WebSocket frame.

### Header Part

| Offset | Size | Field     | Description                                        |
|--------|------|-----------|----------------------------------------------------|
| 0      | 1    | `type`    | `0x01` = header (message metadata)                 |
| 1      | 1    | `subtype` | `0x01` = observed in all captures                  |
| 2      | 4    | `padding` | Always `0x00000000` in all observed frames         |
| 6      | 2    | `length`  | Big-endian uint16, length of the JSON that follows |

Followed by `length` bytes of UTF-8 JSON containing the message metadata.

### Body Part

| Offset | Size | Field     | Description                                               |
|--------|------|-----------|-----------------------------------------------------------|
| 0      | 1    | `type`    | `0x02` = body (payload)                                   |
| 1      | 1    | `subtype` | `0x01` = JSON payload, `0x02` = plain text (log messages) |
| 2      | 4    | `padding` | Always `0x00000000` in all observed frames                |
| 6      | 2    | `length`  | Big-endian uint16, length of the JSON that follows        |

Followed by `length` bytes of UTF-8 JSON (or plain text for log messages).

> **Large bodies**: When the body exceeds 65,535 bytes (uint16 max), the `length` field contains the size of the first
> chunk, and the remaining data continues as raw bytes to the end of the WebSocket frame. The WebSocket frame's own
> payload length (which supports 64-bit) carries the total size. The largest observed `configure` body was ~260KB.

### Example: `enableUpdatesChannel` (raw hex)

```
01 01 00 00 00 00 00 78   ← header part: type=0x01, sub=0x01, len=120
{"timestamp":1782149129925,"type":"request","action":"enableUpdatesChannel",
 "id":"71ce354b-1970-439c-8e13-9230fd0eb3d2"}

02 01 00 00 00 00 00 56   ← body part: type=0x02, sub=0x01, len=86
{"uri":"wss://192.168.0.4:7442",
 "lastUpdateId":"53704449-963a-4ab6-afc8-a7a88b3946db"}
```

### Header JSON

The header JSON contains message metadata. Three types were observed:

**Request:**

```json
{
  "timestamp": 1782149129925,
  "type": "request",
  "action": "enableUpdatesChannel",
  "id": "71ce354b-1970-439c-8e13-9230fd0eb3d2"
}
```

**Response:**

```json
{
  "type": "response",
  "id": "71ce354b-1970-439c-8e13-9230fd0eb3d2",
  "timestamp": 1782149130131,
  "error": "",
  "errorCode": 0
}
```

**Log (device -> controller):**

```json
{
  "id": "9cb78e5a-661b-49d6-86e7-a424edea5710",
  "timestamp": 1782149129936,
  "type": "log",
  "level": "info"
}
```

The `id` field is a UUID used for correlation between request and response. The `timestamp` is in milliseconds; values
are consistent with Unix epoch.

## Messages

### `getInfo` (Controller -> Viewer)

Requests device information. First message sent after WebSocket connection.

**Header:**

```json
{
  "timestamp": 1782149129201,
  "type": "request",
  "action": "getInfo",
  "id": "2a2ef5e4-3bfa-4f0c-8799-632b053ae825"
}
```

**Body:**

```json
{}
```

**Response** (viewer -> controller):

**Header:**

```json
{
  "type": "response",
  "id": "2a2ef5e4-3bfa-4f0c-8799-632b053ae825",
  "timestamp": 1782149129413,
  "error": "",
  "errorCode": 0
}
```

**Body:**

```json
{
  "mac": "E4388334091E",
  "type": "UP Viewport",
  "version": "1.4.33",
  "sw_version": "up-viewport-1.4.33",
  "uptime": 143,
  "network": {
    "linkSpeedMbps": 1000
  }
}
```

### `getConsoleInfo` (Viewer -> Controller)

The device requests console information. This was sent nearly simultaneously with the controller's `getInfo` request.

**Header:**

```json
{
  "type": "request",
  "id": "e0d341e8-fcd1-462c-814e-3653829a8101",
  "timestamp": 1782149129053,
  "action": "getConsoleInfo"
}
```

**Body:**

```json
{}
```

**Response:**

**Header:**

```json
{
  "timestamp": 1782149129720,
  "id": "e0d341e8-fcd1-462c-814e-3653829a8101",
  "type": "response",
  "errorCode": 0,
  "error": ""
}
```

**Body:**

```json
{
  "consoleId": "53540ea4-b520-512c-af90-ef08f10eb2aa",
  "consoleName": "UNVR"
}
```

### `networkStatus` (Controller -> Viewer)

**Header:**

```json
{
  "timestamp": 1782149129747,
  "type": "request",
  "action": "networkStatus",
  "id": "128c0c0d-8812-48cf-b12f-4b1ad103723d"
}
```

**Body:**

```json
{}
```

**Response body:**

```json
{
  "linkSpeedMbps": 1000
}
```

### `changeUserPassword` (Controller -> Viewer)

The controller pushes a password change for the device user.

**Header:**

```json
{
  "timestamp": 1782149129756,
  "type": "request",
  "action": "changeUserPassword",
  "id": "ff299a87-f42a-4e4d-ab8e-229fc384fdb3"
}
```

**Body:**

```json
{
  "username": "ui",
  "passwordOld": "ui",
  "passwordNew": "8VhFT9rTTzjwnEF9lIMG"
}
```

**Response body:**

```json
{}
```

### `configure` (Controller -> Viewer)

The controller pushes the liveview configuration and camera list. This is the core message - it tells the viewer what to
display.

**Header:**

```json
{
  "timestamp": 1782149129885,
  "type": "request",
  "action": "configure",
  "id": "f35cdd3b-8c80-46b7-b4e6-f3d6d61be68a"
}
```

**Body:**

Two `configure` payloads were captured - one for a 16-camera grid layout and one for a 7-camera custom layout.

#### Top-level fields

| Field            | Value         | Notes                     |
|------------------|---------------|---------------------------|
| `name`           | `UP Viewport` | Viewer's display name     |
| `nvr`            | `UNVR4`       | NVR model                 |
| `streamProtocol` | `wss`         |                           |
| `streamPort`     | `7446`        | Livestream WebSocket port |

#### `liveview`

| Field       | Value       | Notes                                                  |
|-------------|-------------|--------------------------------------------------------|
| `name`      | `View Port` |                                                        |
| `isDefault` | `false`     |                                                        |
| `isGlobal`  | `false`     |                                                        |
| `layout`    | `7`         | Layout type code - see [layout reference](./layout.md) |
| `slots`     | [...]       | See below.                                             |
| `owner`     | (present)   |                                                        |
| `id`        | (present)   |                                                        |
| `modelKey`  | (present)   |                                                        |

Each slot:

```json
{
  "cameras": [
    "63406125012bbf03e70003f0"
  ],
  "cycleMode": "time",
  "cycleInterval": 10
}
```

- `cameras`: array of camera IDs (one camera per slot in both captures)
- `cycleMode`: `"time"` or `"motion"` - observed values
- `cycleInterval`: cycling interval (observed: `10`; unit not confirmed)

#### `cameras`

Array of full camera serializations. Below is the complete JSON for the first camera (a UVC G5 Bullet):

```json
{
  "accessDeviceMetadata": {
    "connectedSince": null,
    "disableRecordingByDefault": false,
    "micVolume": 100,
    "featureFlags": {
      "supportLivestream": false,
      "supportUnlock": false,
      "supportMicManagement": false
    },
    "channels": [],
    "pairedInfo": {
      "name": null,
      "uri": null,
      "guid": null
    },
    "talkbackSettings": [],
    "doorInfo": {
      "lockState": null,
      "canLock": false
    },
    "ledSettings": {
      "isEnabled": true
    },
    "speakerSettings": {
      "areSystemSoundsEnabled": true
    }
  },
  "accessMethodSettings": {
    "methods": []
  },
  "activePatrolSlot": null,
  "aiPortCapacityPoints": 0.25,
  "aiPortCompatibleResolutions": [
    "HD",
    "2K"
  ],
  "aiPortCompatibleResolutionsInHallway": [
    "HD"
  ],
  "alarms": {
    "lensThermal": 0,
    "tiltThermal": 0,
    "panTiltMotorFaults": [],
    "autoTrackingThermalThresholdReached": false,
    "lensThermalThresholdReached": false,
    "motorOverheated": false
  },
  "anonymousDeviceId": "3979d045-b707-5741-8f1c-b94cd4369a84",
  "apMac": null,
  "apMgmtIp": null,
  "apRssi": null,
  "audioBitrate": 64000,
  "audioSettings": {
    "style": [
      "nature"
    ]
  },
  "autoRetentionLqMs": null,
  "autoRetentionMs": null,
  "brightnessSettings": {
    "brightness": 36,
    "autoBrightness": true
  },
  "canAdopt": false,
  "canCreateAccessEvent": false,
  "canManage": false,
  "channels": [
    {
      "id": 0,
      "videoId": "video1",
      "name": "High",
      "enabled": true,
      "isRtspEnabled": false,
      "rtspAlias": null,
      "isInternalRtspEnabled": false,
      "internalRtspAlias": null,
      "width": 2688,
      "height": 1512,
      "fps": 30,
      "bitrate": 7000000,
      "minBitrate": 2000000,
      "maxBitrate": 10000000,
      "minClientAdaptiveBitRate": 0,
      "minMotionAdaptiveBitRate": 2000000,
      "fpsValues": [
        1,
        2,
        3,
        4,
        5,
        6,
        8,
        9,
        10,
        12,
        15,
        16,
        18,
        20,
        24,
        25,
        30
      ],
      "idrInterval": 5,
      "autoFps": true,
      "autoBitrate": true,
      "validBitrateRangeMargin": 1000000
    },
    {
      "id": 1,
      "videoId": "video2",
      "name": "Medium",
      "enabled": true,
      "isRtspEnabled": true,
      "rtspAlias": "KOGdikjyy2lNXqdz",
      "isInternalRtspEnabled": false,
      "internalRtspAlias": null,
      "width": 1280,
      "height": 720,
      "fps": 30,
      "bitrate": 1400000,
      "minBitrate": 750000,
      "maxBitrate": 2000000,
      "minClientAdaptiveBitRate": 150000,
      "minMotionAdaptiveBitRate": 750000,
      "fpsValues": [
        1,
        2,
        3,
        4,
        5,
        6,
        8,
        9,
        10,
        12,
        15,
        16,
        18,
        20,
        24,
        25,
        30
      ],
      "idrInterval": 5,
      "autoFps": true,
      "autoBitrate": true,
      "validBitrateRangeMargin": 500000
    },
    {
      "id": 2,
      "videoId": "video3",
      "name": "Low",
      "enabled": true,
      "isRtspEnabled": false,
      "rtspAlias": null,
      "isInternalRtspEnabled": false,
      "internalRtspAlias": null,
      "width": 640,
      "height": 360,
      "fps": 30,
      "bitrate": 310000,
      "minBitrate": 210000,
      "maxBitrate": 1000000,
      "minClientAdaptiveBitRate": 0,
      "minMotionAdaptiveBitRate": 210000,
      "fpsValues": [
        1,
        2,
        3,
        4,
        5,
        6,
        8,
        9,
        10,
        12,
        15,
        16,
        18,
        20,
        24,
        25,
        30
      ],
      "idrInterval": 5,
      "autoFps": true,
      "autoBitrate": true,
      "validBitrateRangeMargin": 100000
    }
  ],
  "chimeDuration": 0,
  "clarityZones": [],
  "connectedSince": null,
  "connectionHost": "192.168.0.4",
  "currentResolution": "2K",
  "displayName": "Patio Camera",
  "doorbellSession": {
    "sessionId": null,
    "status": null,
    "directoryId": null
  },
  "downScaleMode": 0,
  "elementInfo": null,
  "enableNfc": false,
  "excludeZones": [],
  "extendedAiFeatures": {
    "smartDetectTypes": []
  },
  "faceUnlockSettings": {
    "licenseConfigured": false,
    "faceDetectionSensitive": "far",
    "lastUpdateTime": 0
  },
  "featureFlags": {
    "canAdjustIspSettings": true,
    "canAdjustIrLedLevel": false,
    "canAdjustSpeakerVolume": false,
    "maxScaleDownLevel": 1,
    "downScaleResolutions": [
      [
        2688,
        1512
      ],
      [
        1920,
        1080
      ]
    ],
    "downScaleLevels": null,
    "canMagicZoom": false,
    "canOpticalZoom": false,
    "canTouchFocus": false,
    "hasAccelerometer": false,
    "hasVerticalFlip": true,
    "hasHorizontalFlip": true,
    "hasAec": false,
    "hasBluetooth": false,
    "hasChime": false,
    "hasExternalIr": false,
    "hasIcrSensitivity": true,
    "hasInfrared": true,
    "hasLdc": false,
    "hasLedIr": true,
    "hasLedStatus": false,
    "hasLineIn": false,
    "hasMic": true,
    "hasPrivacyMask": true,
    "hasRtc": false,
    "hasSdCard": false,
    "hasSpeaker": false,
    "hasWifi": false,
    "hasHdr": true,
    "hasWdr": true,
    "hasAutoICROnly": true,
    "videoModes": [
      "default",
      "sport",
      "slowShutter"
    ],
    "videoModeMaxFps": [
      30,
      30,
      20
    ],
    "hasMotionZones": true,
    "hasLcdScreen": false,
    "hasFingerprintSensor": false,
    "hasFisheye": false,
    "mountPositions": [],
    "smartDetectTypes": [
      "person",
      "vehicle",
      "animal"
    ],
    "smartDetectAudioTypes": [
      "alrmSmoke",
      "alrmCmonx",
      "alrmBabyCry",
      "alrmSpeak"
    ],
    "supportDoorAccessConfig": false,
    "supportNfc": false,
    "supportLpDetectionWithoutVehicle": false,
    "supportCustomRingtone": false,
    "supportPtzTrackingTimeout": false,
    "supportPtzVehicleTracking": false,
    "lensType": null,
    "lensModel": null,
    "motionAlgorithms": [
      "enhanced"
    ],
    "hasMotionDetection": true,
    "hasSquareEventThumbnail": true,
    "hasPackageCamera": false,
    "audio": [],
    "audioCodecs": [
      "aac",
      "opus"
    ],
    "videoCodecs": [
      "h264",
      "h265",
      "mjpg"
    ],
    "audioStyle": [
      "nature",
      "noiseReduced"
    ],
    "isDoorbell": false,
    "isPtz": false,
    "presetMinDuration": null,
    "hasColorLcdScreen": false,
    "hasLiveviewTracking": false,
    "hasLineCrossing": true,
    "hasLineCrossingCounting": false,
    "hasFlash": false,
    "flashRange": null,
    "hasLuxCheck": true,
    "presetTour": false,
    "hasEdgeRecording": false,
    "hasLprReflex": false,
    "hasSmokeCover": false,
    "streamEncryptable": true,
    "hasManualPersonOfInterest": false,
    "hasPackageZoneSupportForPrimaryLens": false,
    "hasPackageZoneSupportForSecondaryLens": false,
    "hasHallwayMode": true,
    "hasHallwayModeHdrOnRequired": false,
    "hallwayModeWarningRequired": true,
    "supportFullHdSnapshot": false,
    "supportMinMotionAdaptiveBitrate": true,
    "hasTamperDetection": false,
    "supportLocate": false,
    "clarityZones": null,
    "excludeZones": {
      "maxZones": 16,
      "rectangleOnly": true
    },
    "hasSmartZoom": false,
    "hasOptimizeIr": false,
    "verticalFlipWarning": false,
    "stitchDistance": {
      "support": false
    },
    "videoInputModes": [],
    "storage": {
      "sdSlotCount": 0,
      "ssdSlotCount": 0
    },
    "videoDeviceCount": 1,
    "privacyMaskCapability": {
      "maxMasks": 16,
      "rectangleOnly": false
    },
    "focus": {
      "steps": {
        "max": null,
        "min": null,
        "step": null
      },
      "degrees": {
        "max": null,
        "min": null,
        "step": null
      }
    },
    "pan": {
      "steps": {
        "max": null,
        "min": null,
        "step": null
      },
      "degrees": {
        "max": null,
        "min": null,
        "step": null
      }
    },
    "tilt": {
      "steps": {
        "max": null,
        "min": null,
        "step": null
      },
      "degrees": {
        "max": null,
        "min": null,
        "step": null
      }
    },
    "zoom": {
      "ratio": 1,
      "steps": {
        "max": null,
        "min": null,
        "step": null
      },
      "degrees": {
        "max": null,
        "min": null,
        "step": null
      }
    },
    "hotplug": {
      "audio": null,
      "video": null,
      "standaloneAdoption": false,
      "sdCardAttached": false,
      "extender": {
        "isAttached": false,
        "hasFlash": false,
        "flashRange": null,
        "hasIR": false,
        "hasRadar": false,
        "radarRangeMax": null,
        "radarRangeMin": null
      }
    },
    "reader": {
      "supportStatusLed": false,
      "supportWelcomeLed": false,
      "supportFloodLed": false,
      "supportAutoBrightness": false,
      "supportAccessMethods": [],
      "supportDoorbellTriggerMethod": false,
      "supportInterfaceDirectory": false,
      "supportInterfaceLayout": false,
      "support2fa": false,
      "supportSsh": true,
      "supportLocate": false,
      "supportDoorDirection": false,
      "supportCallerManager": false,
      "supportGreetings": false,
      "supportShowUnlockSchedule": false,
      "canAdjustBrightness": false,
      "supportMic": true,
      "supportSpeaker": false,
      "supportAdjustSpeakerVolume": false,
      "supportShowInterfaceImage": false,
      "supportGifInterfaceImage": false,
      "supportAutoTurnOffDisplay": false,
      "supportShowHeading": false,
      "supportShowStatusBar": false,
      "supportInterfaceDesigner": false,
      "supportTwilioSip": false,
      "supportAudioCodecs": [],
      "supportManualDownloadSupportFile": false,
      "supportManualFirmwareUpdate": false,
      "supportStreamEncryption": true,
      "supportGateStop": false,
      "supportPinCodeShuffle": false,
      "supportStatusSound": false
    },
    "hasSmartDetect": true
  },
  "fingerprintSettings": {
    "enable": false,
    "enablePrintLatency": false,
    "mode": "identify",
    "reportFingerTouch": false,
    "reportCaptureComplete": false
  },
  "fingerprintState": {
    "fingerprintId": null,
    "status": null,
    "progress": null,
    "total": 0,
    "free": 0
  },
  "firmwareBuild": "bae2f04.260519.829",
  "firmwareVersion": "5.3.90",
  "fwUpdateState": null,
  "globalAlarmManagerScopeNames": [
    "scope_all_devices",
    "scope_all_cameras",
    "scope_all_smart_cameras",
    "scope_all_smart_cameras_with_zones",
    "scope_all_smart_cameras_with_microphone",
    "scope_all_ui_cameras"
  ],
  "greetingSettings": {
    "greetingText": "welcome",
    "greetingBroadcastName": "firstNameOnly"
  },
  "guid": "afabdce0-f8eb-42d2-85fe-fc77f2c37bf6",
  "hallwayMode": "disabled",
  "hardwareRevision": 14,
  "hasPackageCamera": false,
  "hasRecordingStarted": true,
  "hasRecordings": true,
  "hasSpeaker": false,
  "hasWifi": false,
  "hdrMode": true,
  "hdrType": "auto",
  "homekitAccessoryId": null,
  "homekitSettings": {
    "talkbackSettingsActive": false,
    "streamInProgress": false,
    "microphoneMuted": false,
    "recordingActive": false,
    "doorbellMuted": null,
    "speakerMuted": false
  },
  "host": "192.168.0.222",
  "hqBytesPerDay": 23426916820,
  "hubMac": null,
  "id": "65c5238f02ccde03e406583e",
  "interfaceSettings": {
    "logoImageId": null,
    "bgImageId": null,
    "showLogo": true,
    "heading": null,
    "subHeading": null,
    "layout": "horizontal",
    "callMethod": "swipe",
    "showTime": true,
    "showWeather": true
  },
  "is2K": true,
  "is4K": false,
  "isAccessDevice": false,
  "isAccessFloodlightTriggerEnabled": false,
  "isAdopted": true,
  "isAdoptedByAccessApp": false,
  "isAdoptedByOther": false,
  "isAdopting": false,
  "isAttemptingToConnect": false,
  "isBlockedByArmMode": false,
  "isConnected": false,
  "isDark": false,
  "isDeleting": false,
  "isDownloadingFW": false,
  "isExtenderInstalledEver": false,
  "isIntercom": false,
  "isLiveHeatmapEnabled": false,
  "isManaged": true,
  "isMicEnabled": true,
  "isMissingRecordingDetected": false,
  "isMotionDetected": false,
  "isPairedWithAiPort": false,
  "isPoorNetwork": false,
  "isProbingForWifi": false,
  "isProvisioned": true,
  "isReaderPro": false,
  "isRebooting": false,
  "isRecording": true,
  "isRecordingsPaused": false,
  "isRecordingsPausedChangedAt": null,
  "isRestoring": false,
  "isReverting": false,
  "isSmartDetected": false,
  "isSshEnabled": false,
  "isThirdPartyCamera": false,
  "isUpdating": false,
  "isWaterproofCaseAttached": false,
  "isWirelessUplinkEnabled": true,
  "ispSettings": {
    "aeMode": "auto",
    "irLedMode": "auto",
    "irLedLevel": 255,
    "wdr": 1,
    "icrSensitivity": 0,
    "icrSwitchMode": "sensitivity",
    "icrCustomValue": 2,
    "brightness": 50,
    "contrast": 50,
    "hue": 50,
    "saturation": 50,
    "sharpness": 50,
    "denoise": 50,
    "isColorNightVisionEnabled": false,
    "spotlightDuration": 15,
    "isFlippedVertical": false,
    "isFlippedHorizontal": false,
    "isAutoRotateEnabled": false,
    "isLdcEnabled": true,
    "is3dnrEnabled": true,
    "isExternalIrEnabled": false,
    "isAggressiveAntiFlickerEnabled": false,
    "isPauseMotionEnabled": false,
    "dZoomCenterX": 50,
    "dZoomCenterY": 50,
    "dZoomScale": 0,
    "dZoomStreamId": 4,
    "focusMode": "ztrig",
    "focusPosition": 0,
    "touchFocusX": 1001,
    "touchFocusY": 1001,
    "zoomPosition": 0,
    "mountPosition": "ceiling",
    "hdrMode": "normal",
    "sceneMode": "auto",
    "isSmokeCoverModeEnabled": false
  },
  "lastDisconnect": 1780376971260,
  "lastMotion": 1780361494001,
  "lastRing": null,
  "lastSeen": 1780376970924,
  "latestFirmwareSizeBytes": null,
  "latestFirmwareVersion": null,
  "lcdMessage": {},
  "ledSettings": {
    "isEnabled": true,
    "welcomeLed": true,
    "floodLed": true
  },
  "lenses": [],
  "lowMemoryDisabledProcesses": null,
  "lqBytesPerDay": 2105456520,
  "mac": "F4E2C67804A5",
  "marketName": "G5 Bullet",
  "micVolume": 100,
  "minFirmwareVersion": "5.1.240",
  "modelKey": "camera",
  "motionZones": [
    {
      "id": 1,
      "name": "Default",
      "color": "#AB46BC",
      "points": [
        [
          0,
          0
        ],
        [
          1,
          0
        ],
        [
          1,
          1
        ],
        [
          0,
          1
        ]
      ],
      "sensitivity": 50,
      "isTriggerLightEnabled": true,
      "mergeId": null
    }
  ],
  "name": "Patio Camera",
  "needUpdateBeforeAdoption": false,
  "nfcSettings": {
    "enableNfc": false,
    "supportThirdPartyCard": false
  },
  "nfcState": {
    "lastSeen": null,
    "mode": "disabled",
    "cardId": null,
    "isUACard": false
  },
  "nvrMac": "602232609D4F",
  "optimizeIrSettings": {
    "mode": "disable",
    "irZones": []
  },
  "osdSettings": {
    "isNameEnabled": true,
    "isDateEnabled": true,
    "isLogoEnabled": true,
    "isDebugEnabled": false,
    "overlayLocation": "topLeft"
  },
  "parentCameraGroupId": null,
  "phyRate": null,
  "pinCodeSettings": {
    "pinCodeLengthRange": "4",
    "pinCodeShuffle": false
  },
  "platform": "sav530q",
  "previousFirmwareUrl": null,
  "previousFirmwareVersion": "5.3.89",
  "privacyZones": [],
  "ptz": {
    "returnHomeAfterInactivityMs": 30000,
    "recentAutoHomeReturnAt": null,
    "pauseAutoTrackingUntilTs": null,
    "recentMoveAutoTrackResumeAtTs": null
  },
  "ptzControlEnabled": true,
  "readerSettings": {
    "screenOffTimeout": "auto",
    "allowThirdPartyNfcCards": true,
    "language": "",
    "unlockDuration": 5,
    "doorName": null,
    "doorId": null,
    "doorEntryMethod": "in"
  },
  "receiverGroups": [],
  "recordingPath": "/srv/unifi-protect/video",
  "recordingPathFailedAt": null,
  "recordingPathSettings": {
    "storageConsoleIndex": 0,
    "isAutoFailoverEnabled": false,
    "failoverTimeoutMs": null
  },
  "recordingSchedulesV2": [],
  "recordingSettings": {
    "prePaddingSecs": 2,
    "postPaddingSecs": 2,
    "smartDetectPrePaddingSecs": 2,
    "smartDetectPostPaddingSecs": 2,
    "accessEventPrePaddingSecs": 2,
    "accessEventPostPaddingSecs": 2,
    "minMotionEventTrigger": 1000,
    "endMotionEventDelay": 3000,
    "suppressIlluminationSurge": false,
    "mode": "always",
    "inScheduleMode": "always",
    "outScheduleMode": "never",
    "recordAudio": true,
    "recordVideo": true,
    "geofencing": "off",
    "retentionDurationMs": null,
    "retentionDurationLQMs": null,
    "motionAlgorithm": "enhanced",
    "enableMotionDetection": true,
    "createAccessEvent": true,
    "useNewMotionAlgorithm": true
  },
  "recordingsPausedReason": null,
  "releaseNotePath": "sav530q",
  "rtspClient": null,
  "secondLensSmartDetectZones": [],
  "shortcuts": [
    {
      "id": "9a867381-68ed-4bab-b63c-9131122e5293",
      "placement": {
        "x": 14.548126377663484,
        "y": 11.49500377548151
      },
      "shortcut": {
        "type": "linkedCamera",
        "linkedCameraId": "65c520010247de03e4065494"
      }
    }
  ],
  "skipCameraUpdateDecalListener": false,
  "smartDetectLines": [],
  "smartDetectLoiterZones": [],
  "smartDetectSettings": {
    "objectTypes": [
      "person",
      "vehicle",
      "animal"
    ],
    "autoTrackingObjectTypes": [],
    "autoTrackingWithZoom": true,
    "autoTrackingTimeoutSec": 20,
    "audioTypes": [
      "smoke_cmonx",
      "alrmSmoke",
      "alrmCmonx",
      "alrmBabyCry",
      "alrmSpeak"
    ],
    "enableTamperDetection": false,
    "detectionRange": {
      "max": null,
      "min": null
    }
  },
  "smartDetectZones": [
    {
      "id": 1,
      "name": "Default",
      "color": "#AB46BC",
      "points": [
        [
          0,
          0
        ],
        [
          1,
          0
        ],
        [
          1,
          1
        ],
        [
          0,
          1
        ]
      ],
      "sensitivity": 50,
      "objectTypes": [
        "person",
        "vehicle"
      ],
      "isTriggerLightEnabled": true,
      "source": "unifi-protect",
      "triggerAccessTypes": [],
      "enableAccessLPOnlyMode": false,
      "mergeId": null
    }
  ],
  "speakerSettings": {
    "isEnabled": true,
    "areSystemSoundsEnabled": false,
    "volume": 100,
    "ringVolume": 100,
    "ringtoneId": null,
    "repeatTimes": 1,
    "speakerVolume": 100
  },
  "state": "DISCONNECTED",
  "stats": {
    "wifi": {
      "channel": null,
      "frequency": null,
      "linkSpeedMbps": null,
      "signalQuality": 50,
      "signalStrength": 0
    },
    "video": {
      "recordingStart": 1777248485537,
      "recordingEnd": 1780376909243,
      "recordingStartLQ": 1777248485541,
      "recordingEndLQ": 1780376910081,
      "timelapseStart": 1777248485537,
      "timelapseEnd": 1780376835080,
      "timelapseStartLQ": 1777248485541,
      "timelapseEndLQ": 1780375285061
    },
    "storage": {
      "used": null,
      "rate": null,
      "channelStorage": {
        "0": {
          "rotating": {
            "recordingsSizeBytes": 848256040960,
            "lockedRecordingsSizeBytes": 0
          },
          "timelapse": {
            "recordingsSizeBytes": 1073741824,
            "lockedRecordingsSizeBytes": 0
          }
        },
        "2": {
          "rotating": {
            "recordingsSizeBytes": 76235669504,
            "lockedRecordingsSizeBytes": 0
          },
          "timelapse": {
            "recordingsSizeBytes": 1073741824,
            "lockedRecordingsSizeBytes": 0
          }
        }
      }
    },
    "sdCard": {
      "state": "unmounted",
      "health": null,
      "mounts": [],
      "serial": null,
      "size": null,
      "sdRecordingSupported": null,
      "type": null,
      "slotIdx": null,
      "hotPlugCapable": null,
      "healthStatus": "insufficient_size",
      "usedSize": 0,
      "slotId": "sd"
    },
    "storageSlots": [],
    "edgeRecording": {
      "recordStreamNumber": null,
      "recordMode": "smartDetect",
      "deviceMac": null
    },
    "wifiQuality": 50,
    "wifiStrength": 0,
    "sdCardStorageCapacityMs": null,
    "totalStorageCapacityMs": null
  },
  "stitchDistance": null,
  "stopStreamLevel": null,
  "streamSharing": {
    "enabled": false,
    "token": null,
    "shareLink": null,
    "expires": null,
    "sharedByUserId": null,
    "sharedByUser": null,
    "maxStreams": null
  },
  "streamingChannels": [],
  "supportAiPortResolution": true,
  "supportAiPortResolutionInHallway": true,
  "supportFileCreatedAt": null,
  "supportFileName": null,
  "supportFileState": null,
  "supportUcp4": false,
  "supportedScalingResolutions": [
    "HD",
    "2K"
  ],
  "sysid": null,
  "talkbackSettings": {
    "typeFmt": "aac",
    "typeIn": "serverudp",
    "bindAddr": "0.0.0.0",
    "bindPort": 7004,
    "filterAddr": null,
    "filterPort": null,
    "channels": 1,
    "samplingRate": 22050,
    "bitsPerSample": 16,
    "quality": 100
  },
  "template": null,
  "thirdPartyCameraInfo": {
    "port": null,
    "rtspUrl": null,
    "hasAudio": null,
    "forceTcp": null,
    "enableRtspAudio": null,
    "rtspUrlLQ": null,
    "snapshotUrl": null,
    "errors": [],
    "motionDetection": null
  },
  "tiltLimitsOfPrivacyZones": {
    "side": "top",
    "limit": 0
  },
  "type": "UVC G5 Bullet",
  "upSince": 1780044092057,
  "uplinkDevice": null,
  "uptime": null,
  "userConfiguredAp": false,
  "videoCodec": "h265",
  "videoCodecLastSwitchAt": null,
  "videoCodecState": 0,
  "videoCodecSwitchingSince": null,
  "videoInputMode": "",
  "videoMode": "default",
  "videoReconfigurationInProgress": false,
  "voltage": null,
  "wifiConnectionState": {
    "channel": null,
    "frequency": null,
    "phyRate": null,
    "txRate": null,
    "signalQuality": null,
    "ssid": null,
    "bssid": null,
    "apName": null,
    "experience": null,
    "signalStrength": null,
    "connectivity": null
  },
  "wiredConnectionState": {
    "phyRate": null
  }
}
```

The viewer responds to `configure` with an empty body `{}`.

### `enableUpdatesChannel` (Controller -> Viewer)

Tells the viewer to open a second WebSocket for realtime push events.

**Header:**

```json
{
  "timestamp": 1782149129925,
  "type": "request",
  "action": "enableUpdatesChannel",
  "id": "71ce354b-1970-439c-8e13-9230fd0eb3d2"
}
```

**Body:**

```json
{
  "uri": "wss://192.168.0.4:7442",
  "lastUpdateId": "53704449-963a-4ab6-afc8-a7a88b3946db"
}
```

The viewer then opens a second WebSocket to `wss://192.168.0.4:7442/?lastUpdateId=53704449-963a-4ab6-afc8-a7a88b3946db`
with `Sec-WebSocket-Protocol: updates` and the same `x-ident`, `x-type`, `x-mode` headers as the main connection.

**Response body:**

```json
{}
```

### `getStreamAlias` (Viewer -> Controller)

The viewer requests a stream alias for a specific camera+channel. Sent after receiving `configure` - one request per
camera in the liveview (16 observed for the 16-camera layout, 7 for the 7-camera layout).

**Header:**

```json
{
  "type": "request",
  "id": "517275f2-16ca-4818-bd07-a1d8e019c477",
  "timestamp": 1782149130773,
  "action": "getStreamAlias"
}
```

**Body:**

```json
{
  "camera": "65c5238f02ccde03e406583e",
  "channel": 0,
  "type": "ubv"
}
```

- `camera`: the camera's `id` from the `configure` payload
- `channel`: channel index (only `0` observed in `getStreamAlias` requests)
- `type`: `"ubv"`

**Response body:**

```json
{
  "alias": "I47J3Bo9YYV1vBFh",
  "url": "wss://192.168.0.4:7446/I47J3Bo9YYV1vBFh?type=ubv",
  "rtspUrl": "rtsp://192.168.0.4:7447/I47J3Bo9YYV1vBFh"
}
```

The viewer connects to `url` (port 7446) or `rtspUrl` (port 7447) to receive the camera stream.

### Log Messages (Viewer -> Controller)

The viewer sends Android-style logcat messages to the controller. These are informational.

**Header:**

```json
{
  "id": "3a77c8e1-c1fb-42fb-9685-ff8aefa82bb7",
  "timestamp": 1782149130188,
  "type": "log",
  "level": "info"
}
```

**Body** (subtype `0x02`, plain text):

```
I/LiveViewFragment( 2307): onConfigure: count=16
```

### `update` (Controller -> Viewer, via Updates Channel)

Observed on the updates channel (second WebSocket). These are push notifications for device state changes (e.g., camera
state changes, firmware updates).

**Header:**

```json
{
  "action": "update",
  "newUpdateId": "4889e570-77ca-4d71-8d0c-0667ab8102b",
  "modelKey": "camera",
  "id": "688e2bfe0165bd03e47c7518",
  "mac": "8478482A633E",
  "nvrMac": "602232609D4F",
  "token": null,
  "state": "CONNECTED",
  "modifiedKeys": [
    "videoReconfigurationInProgress",
    "nvrMac"
  ]
}
```

**Body:**

```json
{
  "videoReconfigurationInProgress": false,
  "nvrMac": "602232609D4F"
}
```

## Stream Delivery

The viewer **pulls** streams from the controller. Observed flow:

1. Controller sends `configure` with `liveview.slots[].cameras[]` (camera IDs) and `cameras[]` (full camera
   serializations).
2. Viewer extracts camera IDs from the liveview slots and matches them to the `cameras[]` array.
3. For each camera, viewer sends `getStreamAlias` with `{camera, channel, type: "ubv"}`.
4. Controller responds with `{alias, url, rtspUrl}`.
5. Viewer connects to `url` (`wss://NVR:7446/<alias>?type=ubv`) or `rtspUrl` (`rtsp://NVR:7447/<alias>`) to receive the
   stream.

### Ports

| Port | Protocol | Purpose                                                                    |
|------|----------|----------------------------------------------------------------------------|
| 8080 | TLS      | Device-side API - NVR pushes adoption info to device (content not decoded) |
| 7442 | WSS      | UCP4 device WebSocket (adoption + control) + updates channel               |
| 7446 | WSS      | Livestream WebSocket (stream delivery)                                     |
| 7447 | RTSP     | RTSP livestream                                                            |

## TLS Client Certificate

The `ds` proxy requires a TLS client certificate. It extracts the fingerprint (colon-separated hex, consistent with SHA1
format) and forwards it as the `x-fingerprint` header.

The fingerprints changed between factory resets. The actual certificate subject, key type, and validity period were not
captured - TLS 1.3 encrypts the client certificate in the handshake.

So the protocol appears to use no certificate-pinning and uses self-signed client certificates after adoption.

## Device Type Identification

The device identifies itself via two headers:

| Header    | Value         | Notes                                               |
|-----------|---------------|-----------------------------------------------------|
| `x-sysid` | `0xe980`      | Observed in UNVR logs; not in the plaintext capture |
| `x-type`  | `UP Viewport` | Observed in the plaintext capture.                  |

Known viewer sysids (from UNVR logs):

| Sysid    | Device Type        |
|----------|--------------------|
| `0xe980` | UP Viewport        |
| `0xec65` | UA-Intercom-Viewer |
