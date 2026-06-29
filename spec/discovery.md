# UniFi Protect Device Discovery Protocol (UDP 10001)

> Status: Incomplete - there are likely other commands that can be sent, but the `CMD_INFO` query/response is fully
> decoded.

> Raw frames captured from a real ViewPort (UP Viewport, firmware `1.4.33`) adopting against a UNVR running Protect
> 7.1.83 / UniFi OS 5.1.19. Field semantics cross-validated against decompiled firmware source.

## Overview

Discovery on UDP port 10001 is a query/response protocol driven by the controller (the NVR and/or the
UniFi gateway), not an unsolicited announcement from the device.

1. A controller periodically sends an empty `CMD_INFO` query to the LAN, addressed to both the broadcast
   address (`255.255.255.255:10001`) and the Ubiquiti discovery multicast group (`233.89.188.1:10001`).
2. Every Ubiquiti device that receives the query replies by unicast with a TLV payload describing itself
   (MAC, IP, firmware, model, adoption state, etc.), sent back to the source IP and source port of the query.
3. The device learns the controller's address from the source IP of the query.

The device replies to each controller that queries it. On a network with more than one controller, the device sends its
TLV reply to each of them independently.

## Protocol

### Multicast Membership

Devices join the multicast group `233.89.188.1` (observed via IGMPv3 membership reports to `224.0.0.22`) so they receive
queries sent to the group.

### Device-Side Listen Socket

The device binds UDP port 10001 on all interfaces with the following socket options:

- Join multicast group `233.89.188.1`
- `SO_REUSEADDR = true`
- `SO_TIMEOUT = 5000` (5 seconds)

### Query Validation (Device Side)

The device only responds to a query that satisfies all of:

1. Exactly 4 bytes: `{0x01, 0x00, 0x00, 0x00}` (version `0x01`, command `CMD_INFO`, empty TLV section).
2. Source IP is in a private / test range:
    - `10.0.0.0/8`
    - `172.16.0.0/12`
    - `192.168.0.0/16`
    - `169.254.0.0/16` (link-local)
    - `1.2.3.0/24` (test range)

Frames with a non-empty TLV section are treated as announcements / other commands and do not trigger a response.

### Query

- Direction: `controller -> devices`
- Protocol: UDP
- Source port: ephemeral (the controller uses an ephemeral port per query)
- Destination: sent twice per round - once to `255.255.255.255:10001` (broadcast) and once to
  `233.89.188.1:10001` (multicast), with identical payload and source port
- Frequency: approximately every 10 seconds per controller (observed ~10.5s)

Controllers query the network for UniFi devices and the devices respond. The controller learns about every device that
replies, and each device learns the controller's address from the query's source IP.

**Payload**

The query is a 4-byte header with an empty TLV section:

```
01 00 00 00
```

| Offset | Size | Field   | Value    | Meaning                          |
|--------|------|---------|----------|----------------------------------|
| 0      | 1    | version | `0x01`   | Protocol version                 |
| 1      | 1    | command | `0x00`   | `CMD_INFO` (request device info) |
| 2      | 2    | length  | `0x0000` | TLV section length = 0 (empty)   |

### Response

- Direction: `device -> controller`
- Protocol: UDP
- Source port: `10001` (the device binds and replies from this fixed port)
- Destination: the unicast IP and ephemeral port the query originated from

**Payload**

Each response is a binary payload with a 4-byte header followed by a sequence of TLV (Type-Length-Value) entries.

**Payload Header**

The header layout is the same for queries and responses; the difference is the command byte and whether a TLV section
follows.

| Offset | Size | Field   | Notes                                                                                                                      |
|--------|------|---------|----------------------------------------------------------------------------------------------------------------------------|
| 0      | 1    | version | Observed: `0x01`                                                                                                           |
| 1      | 1    | command | Observed: `0x00` (`CMD_INFO`) in both the query and the response                                                           |
| 2      | 2    | length  | Big-endian uint16. Total length of TLV section (excludes the 4-byte header). `0x0000` for a query; non-zero for a response |

The header length matches the captured frames exactly: pre-adoption `0x00B7` (183) + 4-byte header = 187 bytes;
post-adoption `0x00CA` (202) + 4 = 206 bytes.

### TLV Entry

Each TLV entry has a 3-byte header followed by the value:

| Offset | Size | Field  | Notes                                           |
|--------|------|--------|-------------------------------------------------|
| 0      | 1    | type   | TLV type code                                   |
| 1      | 2    | length | Big-endian uint16, length of the value in bytes |
| 3      | *n*  | value  | `length` bytes                                  |

> The 3-byte TLV header matches the 4-byte payload header's length encoding (big-endian uint16 at offset 2). The
> previously documented `type(1) + reserved_0x00(1) + length_uint8(1)` encoding decodes every captured frame to the
> same bytes - all observed values are shorter than 256 bytes - but is inconsistent with the payload header. The
> `length` is treated as a uint16 here. An implementer should emit `0x00` for the high byte of `length` whenever
> `length <= 255`, which makes the two encodings byte-compatible for all observed frames.

This encoding parses both captured frames cleanly to the exact byte. For example, the first TLV
`01 00 06 e4 38 83 34 09 1e` decodes as type `0x01`, length `0x0006`, value `e4388334091e` (the MAC).

**TLV Types**

The following types are relevant to the ViewPort:

| Type   | Name                | Len (Bytes) | Value Format                                                                                                                                                                                                                                            | When Present |
|--------|---------------------|-------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------------|
| `0x01` | MAC Address         | 6           | Raw MAC bytes                                                                                                                                                                                                                                           | Always       |
| `0x02` | MAC + IP            | 10          | 6-byte MAC + 4-byte IPv4                                                                                                                                                                                                                                | Always       |
| `0x03` | Firmware Version    | var         | ASCII string: `UPV.qcs605.v1.4.33.0.4698daf26.260416.1114`                                                                                                                                                                                              | Always       |
| `0x0A` | Uptime              | 4           | Big-endian uint32, seconds since boot                                                                                                                                                                                                                   | Always       |
| `0x0B` | Hostname            | var         | ASCII string: `UP Viewport`                                                                                                                                                                                                                             | Always       |
| `0x0C` | Platform            | var         | ASCII string: `UP Viewport`                                                                                                                                                                                                                             | Always       |
| `0x17` | Is Default          | 4           | uint32: `0x00000001` = unadopted, `0x00000000` = adopted. `is_default = !isAdopted ? 1 : 0`.                                                                                                                                                            | Always       |
| `0x2C` | Default Credentials | 1           | Bitfield: bit 0 = `ubnt` supported, bit 1 = `ui` supported. Value `0x03` = both. These are the credentials the device accepts on its management API (port 8080); the password is replaced by the Protect Console's device password soon after adoption. | Always       |
| `0x10` | System ID           | 2           | `0x80E9` - byte-swap of `0xE980`, which is sent in the `x-sysid` header over WebSocket. This is the device type id; e.g. `0xec65` is the UA-Intercom-Viewer.                                                                                            | Always       |
| `0x0F` | Signal              | 4           | `0x00011F90` (constant `73360`). Purpose unconfirmed.                                                                                                                                                                                                   | Always       |
| `0x20` | Anonymous ID        | 36          | ASCII UUID string                                                                                                                                                                                                                                       | Always       |
| `0x2B` | GUID                | 16          | Binary 16 bytes of `1385fe74-06ad-496f-933e-c1785e3d7947`. Hardcoded into the Protect ViewPort's APK - identical for all ViewPort devices on this firmware.                                                                                             | Always       |
| `0x26` | Controller ID       | 16          | Binary 16-byte NVR hardware ID. Only included when adopted (the stored NVR hardware ID is non-null).                                                                                                                                                    | Adopted only |

> Not observed on the device: types `0x13` (Serial) and `0x14` (Model / Full Name) are referenced in
> the [HN thread](https://news.ycombinator.com/item?id=47308278) and the camera discovery protocol but do not appear in
> any captured ViewPort frame.

### TLV Construction Order

The device builds TLVs in this order:

1. **Common TLVs:** MAC (`0x01`), MAC+IP (`0x02`), Firmware (`0x03`), Uptime (`0x0A`), Hostname (`0x0B`),
   Platform (`0x0C`), Is Default (`0x17`), Default Credentials (`0x2C`).
2. **Device-specific TLVs:** System ID (`0x10`), Signal (`0x0F`), Anonymous ID (`0x20`), GUID (`0x2B`),
   Controller ID (`0x26` if adopted).

## Examples

### Example: Cold Discovery

The following sequence is from a capture taken on the NVR (`192.168.0.4`) immediately after a physical
factory reset of the ViewPort (`192.168.0.201`). At this point the device has no stored controller address
and `is_default = 0x01`. Times are relative seconds.

```
190.479371  192.168.0.4 -> 233.89.188.1:10001     CMD_INFO query, src port 35326, payload 01000000
190.479402  192.168.0.4 -> 255.255.255.255:10001  CMD_INFO query, src port 35326, payload 01000000  (same round, broadcast)
190.503163  ARP  Who has 192.168.0.4? Tell 192.168.0.201
190.503183  ARP  192.168.0.4 is at 60:22:32:60:9d:4f
190.503783  192.168.0.201:10001 -> 192.168.0.4:35326   TLV response (187 bytes, is_default=0x01)
```

Points an implementer should note:

- The device replied to the query's source port (`35326`), from its own fixed source port `10001`.
- The device had no prior knowledge of `192.168.0.4`. The controller IP is learned from the query.
- The same query round is answered by every Ubiquiti device on the segment; each unicasts its own TLV response to the
  controller's source port.

### Example: Frame Variants

Two frame variants were captured, differing in size and content:

### Variant A: Pre-adoption (187 bytes)

Sent when the device is in factory-default state (`is_default = 0x01`). This is the frame that causes the device to
appear in the Protect pending-adoption list.

### Variant B: Post-adoption (206 bytes)

Sent after adoption is complete (`is_default = 0x00`). Includes an additional TLV (`0x26`) containing a 16-byte value (
`53540ea4b520512caf90ef08f10eb2aa`) that matches a value observed in UNVR logs.

## Example: Raw Frames

### Frame 1 - Pre-adoption (187 bytes)

Captured at t+28.52s (before WebSocket adoption at t+185s).

```
010000b7010006e4388334091e02000ae4388334091ec0a800c903002a5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e313131340a00040000004e0b000b55502056696577706f72740c000b55502056696577706f7274170004000000012c00010310000280e90f000400011f9020002437663963393061322d383135322d356436332d323134622d6439366436643839346231662b00101385fe7406ad496f933ec1785e3d7947
```

**Decoded**:

> Grouped as `type reserved length | value` (the 3-byte TLV header, then value):

```
01 00 00 b7                                                                                    # header: version=01 command=00 length=0x00b7 (183)
01 00 06 e4388334091e                                                                          # 0x01 MAC
02 00 0a e4388334091e c0a800c9                                                                 # 0x02 MAC+IP
03 00 2a 5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e31313134  # 0x03 firmware
0a 00 04 0000004e                                                                              # 0x0A uptime (78s)
0b 00 0b 55502056696577706f7274                                                                # 0x0B hostname "UP Viewport"
0c 00 0b 55502056696577706f7274                                                                # 0x0C platform "UP Viewport"
17 00 04 00000001                                                                              # 0x17 is_default = 1 (unadopted)
2c 00 01 03                                                                                    # 0x2C default credentials: ubnt+ui
10 00 02 80e9                                                                                  # 0x10 system id (0x80E9)
0f 00 04 00011f90                                                                              # 0x0F signal (constant)
20 00 24 37663963393061322d383135322d356436332d323134622d643936643664383934623166              # 0x20 anonymous id (UUID)
2b 00 10 1385fe7406ad496f933ec1785e3d7947                                                      # 0x2B GUID (hardcoded)
```

### Frame 2 - Post-adoption (206 bytes)

Captured at t+186.07s (after WebSocket adoption completed at t+185s).

```
010000ca010006e4388334091e02000ae4388334091ec0a800c903002a5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e313131340a0004000000590b000b55502056696577706f72740c000b55502056696577706f7274170004000000002c00010310000280e90f000400011f9020002437663963393061322d383135322d356436332d323134622d6439366436643839346231662b00101385fe7406ad496f933ec1785e3d794726001053540ea4b520512caf90ef08f10eb2aa
```

**Decoded**:

> Grouped as `type reserved length | value` (the 3-byte TLV header, then value):

```
01 00 00 ca                                                                                    # header: version=01 command=00 length=0x00ca (202)
01 00 06 e4388334091e                                                                          # 0x01 MAC
02 00 0a e4388334091e c0a800c9                                                                 # 0x02 MAC+IP
03 00 2a 5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e31313134  # 0x03 firmware
0a 00 04 00000059                                                                              # 0x0A uptime (89s)
0b 00 0b 55502056696577706f7274                                                                # 0x0B hostname "UP Viewport"
0c 00 0b 55502056696577706f7274                                                                # 0x0C platform "UP Viewport"
17 00 04 00000000                                                                              # 0x17 is_default = 0 (adopted)
2c 00 01 03                                                                                    # 0x2C default credentials: ubnt+ui
10 00 02 80e9                                                                                  # 0x10 system id (0x80E9)
0f 00 04 00011f90                                                                              # 0x0F signal (constant)
20 00 24 37663963393061322d383135322d356436332d323134622d643936643664383934623166              # 0x20 anonymous id (UUID)
2b 00 10 1385fe7406ad496f933ec1785e3d7947                                                      # 0x2B GUID (hardcoded)
26 00 10 53540ea4b520512caf90ef08f10eb2aa                                                      # 0x26 controller id (adopted only)
```

### Example Fields Annotated

Byte-exact decode of the raw frames above using the [TLV encoding](#tlv-entry) (1-byte type, 2-byte big-endian length,
then value). Offsets are byte offsets into the frame, pointing at the TLV type byte; `value offset` = type offset + 3.

| Offset | Type   | Len | Hex Value                                                                              | Decoded Value                                                    |
|--------|--------|-----|----------------------------------------------------------------------------------------|------------------------------------------------------------------|
| 4      | `0x01` | 6   | `e4388334091e`                                                                         | MAC: `E4:38:83:34:09:1E`                                         |
| 13     | `0x02` | 10  | `e4388334091ec0a800c9`                                                                 | MAC + IP: `E4:38:83:34:09:1E` @ `192.168.0.201`                  |
| 26     | `0x03` | 42  | `5550562e7163733630352e76312e342e33332e302e3436393864616632362e3236303431362e31313134` | Firmware: `UPV.qcs605.v1.4.33.0.4698daf26.260416.1114`           |
| 71     | `0x0A` | 4   | `0000004e` (pre) / `00000059` (post)                                                   | Uptime: 78s (pre) / 89s (post)                                   |
| 78     | `0x0B` | 11  | `55502056696577706f7274`                                                               | Hostname: `UP Viewport`                                          |
| 92     | `0x0C` | 11  | `55502056696577706f7274`                                                               | Platform: `UP Viewport`                                          |
| 106    | `0x17` | 4   | `00000001` (pre) / `00000000` (post)                                                   | Is Default: `true` (unadopted) / `false` (adopted)               |
| 113    | `0x2C` | 1   | `03`                                                                                   | Default Credentials: `0x03` = `ubnt` + `ui` supported            |
| 117    | `0x10` | 2   | `80e9`                                                                                 | System ID: `0x80E9` (byte-swap of `0xE980`, the `x-sysid` value) |
| 122    | `0x0F` | 4   | `00011f90`                                                                             | Signal: `0x00011F90` (constant `73360`; purpose unconfirmed)     |
| 129    | `0x20` | 36  | `37663963393061322d383135322d356436332d323134622d643936643664383934623166`             | Anonymous ID: `7f9c90a2-8152-5d63-214b-d96d6d894b1f`             |
| 168    | `0x2B` | 16  | `1385fe7406ad496f933ec1785e3d7947`                                                     | GUID: `1385fe74-06ad-496f-933e-c1785e3d7947` (hardcoded in APK)  |
| 187    | `0x26` | 16  | `53540ea4b520512caf90ef08f10eb2aa`                                                     | Controller ID: NVR hardware ID (post-adoption only)              |

### Uptime

The `0x0A` (uptime) field increments across packets, consistent with a live uptime counter:

| Frame | Capture time (s) | Uptime (s) | Delta                       |
|-------|------------------|------------|-----------------------------|
| 1     | 28.52            | 78         | -                           |
| 2     | 39.02            | 89         | 11                          |
| 3     | 49.54            | 99         | 10                          |
| 4     | 60.04            | 110        | 11                          |
| 12    | 186.07           | 89         | *(reset - device rebooted)* |

### Observations

1. `0x10` (System ID) = `0x80E9` - constant across all captured frames. The bytes `0x80E9` are the byte-swap of
   `0xE980`, which matches the `x-sysid: 0xe980` value seen in the WebSocket adoption (see
   [UCP4 WebSocket spec](./ucp4.md)) that classifies the device as `UP Viewport`. Other known sysids (e.g. `0xec65`
   for the UA-Intercom-Viewer) confirm this is a device-type identifier.

2. `0x17` (Is Default) is `0x00000001` while the device is unadopted (factory default) and `0x00000000` once adopted.
   A factory reset returns it to `0x00000001`. This is how the controller knows whether a responding device is
   available for adoption. Computed as `is_default = !isAdopted ? 1 : 0`.

3. `0x26` (Controller ID) is present only while adopted - the 16-byte value `53540ea4b520512caf90ef08f10eb2aa`
   matches a value observed in UNVR logs. Absent in factory-default responses (`0x17 = 0x01`) and present once
   adopted (`0x17 = 0x00`). Only emitted when the stored NVR hardware ID is non-null.

4. `0x0F` (Signal) = `0x00011F90` - constant across all frames. Value `73360` decimal. Purpose unconfirmed.

5. Hostname and Platform are identical (`UP Viewport`) - the ViewPort does not differentiate these, unlike cameras
   which may have a different hostname from model name.

6. No `0x13` (Serial) or `0x14` (Model) TLVs are present - the ViewPort does not include serial number or full model
   name in its discovery frames.

7. GUID `0x2B` = `1385fe74-06ad-496f-933e-c1785e3d7947` is hardcoded in the APK and identical for all ViewPort
   devices on this firmware. The Anonymous ID `0x20` is a per-device UUID (`7f9c90a2-...` here).
