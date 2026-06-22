# UniFi Protect Device Discovery Protocol (UDP 10001)

> Status: **Partially decoded.** Raw frames captured from a real ViewPort (UP Viewport, firmware `1.4.33`) adopting
> against a UNVR running Protect 7.1.83 / UniFi OS 5.1.19.

## Overview

Ubiquiti Protect devices announce themselves on the local network via UDP port 10001. The device sends unicast packets
to the NVR's IP (not broadcast). The NVR listens on port 10001 and uses the TLV fields to classify the device and add it
to the pending-adoption list.

## Transport

- **Protocol**: UDP
- **Source port**: ephemeral (varies per packet)
- **Destination port**: `10001`
- **Direction**: device -> NVR (unicast to NVR IP, not broadcast)
- **Frequency**: approximately every 10 seconds (observed interval: ~10.5s)

## Frame Format

Each packet is a binary payload with a 4-byte header followed by a sequence of TLV (Type-Length-Value) entries.

### Header (4 bytes)

| Offset | Size | Field   | Notes                                                                |
|--------|------|---------|----------------------------------------------------------------------|
| 0      | 1    | version | Observed: `0x01`                                                     |
| 1      | 1    | flags   | Observed: `0x00`                                                     |
| 2      | 2    | length  | Big-endian. Total length of TLV section (excludes the 4-byte header) |

**Note:** The length field is present in the header but the exact TLV encoding is not yet confirmed. The TLV type and
length fields may be 1-byte or 2-byte each. The raw hex is provided below for analysis.

### TLV Entries

Based on the [HN thread](https://news.ycombinator.com/item?id=47308278) and the camera discovery protocol, the expected
TLV types are:

| Type   | Name                   | Value format                                                |
|--------|------------------------|-------------------------------------------------------------|
| `0x01` | MAC Address            | 6 bytes (raw MAC)                                           |
| `0x02` | MAC + IP               | 10 bytes (6 MAC + 4 IPv4)                                   |
| `0x03` | Firmware Version       | ASCII string                                                |
| `0x0A` | Uptime                 | 4 bytes (big-endian uint32, seconds)                        |
| `0x0B` | Hostname               | ASCII string                                                |
| `0x0C` | Platform / Short Model | ASCII string                                                |
| `0x13` | Serial                 | ASCII string                                                |
| `0x14` | Model / Full Name      | ASCII string                                                |
| `0x17` | Is Default             | 1 byte (0x01 = factory default / unadopted, 0x00 = adopted) |
| `0x0F` | Unknown                | 4 bytes                                                     |
| `0x2C` | Unknown                | 1 byte                                                      |
| `0x26` | NVR Hardware ID        | 16 bytes (observed post-adoption only)                      |

> **Warning**: The TLV type/length field sizes are inferred from byte patterns, not confirmed. The encoding may use
> 1-byte type + 1-byte length, or 2-byte type + 2-byte length, or a different scheme. The raw hex dumps below should be
> used to determine the exact encoding.

## Observed Values (ViewPort)

From the captured ViewPort frames, the following values are identifiable in the raw hex:

| Field             | Value                                                             |
|-------------------|-------------------------------------------------------------------|
| MAC Address       | `E4:38:83:34:09:1E` (appears at offset 8 in the `0x02` TLV)       |
| IP Address        | `192.168.0.201` (`C0 A8 00 C9`)                                   |
| Firmware Version  | `UPV.qcs605.v1.4.33.0.4698daf26.260416.1114`                      |
| Hostname          | `UP Viewport`                                                     |
| Platform          | `UP Viewport`                                                     |
| Is Default (pre)  | `0x01` (factory default / unadopted) - in early frames            |
| Is Default (post) | `0x00` (adopted) - in later frames                                |
| GUID              | `7f9c90a2-8152-5d63-214b-d96d6d894b1f` (36-char ASCII UUID)       |
| NVR Hardware ID   | `53540ea4b520512caf90ef08f10eb2aa` (16 bytes, post-adoption only) |

## Frame Variants

Two frame variants were captured, differing in size and content:

### Variant A: Pre-adoption (187 bytes)

Sent when the device is in factory-default state (`is_default = 0x01`). This is the frame that causes the device to
appear in the Protect pending-adoption list.

### Variant B: Post-adoption (206 bytes)

Sent after adoption is complete (`is_default = 0x00`). Includes an additional TLV (`0x26`) containing a 16-byte value (
`53540ea4b520512caf90ef08f10eb2aa`) that matches a value observed in UNVR logs.

## Raw Frame Examples

### Frame 1 - Pre-adoption (187 bytes)

Captured at t+28.52s (before WebSocket adoption at t+185s).

```
010000b7
0100 06 e4388334091e
0200 0a e4388334091e c0a800c9
0300 2a 5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e31313134
0a00 04 0000004e
0b00 0b 55502056696577706f7274
0c00 0b 55502056696577706f7274
1700 04 00000001
2c00 01 03
1000 02 80e9
0f00 04 00011f90
2000 24 37663963393061322d383135322d356436332d323134622d643936643664383934623166
2b00 10 1385fe7406ad496f933ec1785e3d7947
```

Full hex (single line):

```
010000b7010006e4388334091e02000ae4388334091ec0a800c903002a5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e313131340a00040000004e0b000b55502056696577706f72740c000b55502056696577706f7274170004000000012c00010310000280e90f000400011f9020002437663963393061322d383135322d356436332d323134622d6439366436643839346231662b00101385fe7406ad496f933ec1785e3d7947
```

### Frame 2 - Post-adoption (206 bytes)

Captured at t+186.07s (after WebSocket adoption completed at t+185s).

```
010000ca
0100 06 e4388334091e
0200 0a e4388334091e c0a800c9
0300 2a 5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e31313134
0a00 04 00000059
0b00 0b 55502056696577706f7274
0c00 0b 55502056696577706f7274
1700 04 00000000
2c00 01 03
1000 02 80e9
0f00 04 00011f90
2000 24 37663963393061322d383135322d356436332d323134622d643936643664383934623166
2b00 10 1385fe7406ad496f933ec1785e3d7947
2600 10 53540ea4b520512caf90ef08f10eb2aa
```

Full hex (single line):

```
010000ca010006e4388334091e02000ae4388334091ec0a800c903002a5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e313131340a0004000000590b000b55502056696577706f72740c000b55502056696577706f7274170004000000002c00010310000280e90f000400011f9020002437663963393061322d383135322d356436332d323134622d6439366436643839346231662b00101385fe7406ad496f933ec1785e3d794726001053540ea4b520512caf90ef08f10eb2aa
```

## Field Annotations

Based on manual byte analysis of the raw frames above (TLV with 2-byte type + 2-byte length, big-endian):

| Offset | Type   | Len | Hex Value                                                                              | Decoded Value                                                       |
|--------|--------|-----|----------------------------------------------------------------------------------------|---------------------------------------------------------------------|
| 4      | `0x01` | 6   | `e4388334091e`                                                                         | MAC: `E4:38:83:34:09:1E`                                            |
| 12     | `0x02` | 10  | `e4388334091ec0a800c9`                                                                 | MAC + IP: `E4:38:83:34:09:1E` @ `192.168.0.201`                     |
| 26     | `0x03` | 42  | `5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e31313134` | Firmware: `UPV.qcs605.v1.4.33.0.4698daf26.260416.1114`              |
| 72     | `0x0A` | 4   | `0000004e`                                                                             | Uptime: 78 seconds                                                  |
| 80     | `0x0B` | 11  | `55502056696577706f7274`                                                               | Hostname: `UP Viewport`                                             |
| 95     | `0x0C` | 11  | `55502056696577706f7274`                                                               | Platform: `UP Viewport`                                             |
| 110    | `0x17` | 4   | `00000001` (pre) / `00000000` (post)                                                   | Is Default: `true` / `false`                                        |
| 118    | `0x2C` | 1   | `03`                                                                                   | Unknown (constant `0x03`)                                           |
| 123    | `0x10` | 2   | `80e9`                                                                                 | Unknown (looks like sysid `0xe980` byte-swapped? `0xe980` -> `80e9`) |
| 129    | `0x0F` | 4   | `00011f90`                                                                             | Unknown (constant `0x00011F90` = 73360)                             |
| 137    | `0x20` | 36  | `37663963393061322d383135322d356436332d323134622d643936643664383934623166`             | GUID: `7f9c90a2-8152-5d63-214b-d96d6d894b1f`                        |
| 177    | `0x2B` | 16  | `1385fe7406ad496f933ec1785e3d7947`                                                     | Device ID (binary UUID)                                             |
| 197    | `0x26` | 16  | `53540ea4b520512caf90ef08f10eb2aa`                                                     | NVR Hardware ID (post-adoption only)                                |

### Uptime progression

The `0x0A` (uptime) field increments across packets, consistent with a live uptime counter:

| Frame | Capture time (s) | Uptime (s) | Delta                       |
|-------|------------------|------------|-----------------------------|
| 1     | 28.52            | 78         | -                           |
| 2     | 39.02            | 89         | 11                          |
| 3     | 49.54            | 99         | 10                          |
| 4     | 60.04            | 110        | 11                          |
| 12    | 186.07           | 89         | *(reset - device rebooted)* |

### Key observations

1. **`0x10` field = `80e9`** - This is likely the sysid `0xe980` in little-endian byte order. The `x-sysid: 0xe980`
   header used in the WebSocket adoption (see [UCP4 WebSocket spec](./ucp4-websocket.md)) classifies the device as
   `UP Viewport`. The discovery protocol carries this same sysid, allowing `unifi-core` to classify the device before
   the WebSocket connection.

2. **`0x17` (Is Default) flips from `0x01` to `0x00`** upon adoption - frames before the WebSocket adoption (t+185s)
   have `0x01`, frames after have `0x00`. This tells the NVR whether the device is available for adoption.

3. **`0x26` (NVR Hardware ID) appears only post-adoption** - the 16-byte value `53540ea4b520512caf90ef08f10eb2aa`
   matches a value observed in UNVR logs.

4. **`0x0F` field = `00011f90`** - constant across all frames. Value `73360` decimal. Unknown purpose.

5. **`0x2C` field = `0x03`** - constant. Unknown purpose.

6. **Hostname and Platform are identical** (`UP Viewport`) - the ViewPort doesn't differentiate these, unlike cameras
   which may have a different hostname from model name.

7. **No `0x13` (Serial) or `0x14` (Model) TLVs present** - the ViewPort does not include serial number or full model
   name in its discovery frames.
