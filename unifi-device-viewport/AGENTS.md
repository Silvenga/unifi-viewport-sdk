# unifi-device-viewport

Device-side implementation of a Ubiquiti Protect Viewport like device. Encapsulates:

- The central struct is `ViewPortDevice`, the primary entry point for consumers. It orchestrates the UDP discovery
  responder (port 10001) and the TLS adoption server (port 8080).
- The device is stateful. State is persisted via a consumer-provided `DeviceStorage` impl, allowing the device to
  restore its adopted state across restarts.
- The device starts in factory-default state (adoptable, `is_default=0x01`). When the controller sends a valid adoption
  request, the device generates a client certificate, stores the controller's console ID, marks itself as adopted (
  `is_default=0x00`), and includes the NVR hardware ID in subsequent discovery responses.
- The device password defaults to `ui` (factory default) and can be overridden. The controller can change it via
  `changeUserPassword` (UCP4 protocol, not yet implemented).
- The adoption server listens on port 8080 over TLS 1.3 with a self-signed certificate generated on startup.
- The `/api/adopt` endpoint accepts a POST request with the adoption payload from the controller. On success, a callback
  is invoked with the parsed `AdoptionRequest`.
- The SDK does not auto-detect system info (MAC, IP, hostname). The consumer provides this data via the builder.
- Listens for device discovery requests and responds as a Viewport (joins the discovery multicast address).
- Accepts adoption requests from a controller (via an HTTP/TLS server on port 8080).
- Registers itself with the controller (with the adoption token).
- Requests and receives configuration pushed by the controller (via the UCP4 WebSocket connection).
- Treat network data as untrusted. Validate and warn on unexpected fields per the SDK conventions.
