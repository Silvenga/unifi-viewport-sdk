# unifi-controller

Controller-side implementation of the Ubiquiti Protect adoption flow.

- The controller pushes adoption info to a discovered device via HTTPS POST to `https://<device>:8080/api/adopt`.
- The adoption payload includes the controller's WebSocket endpoint, adoption token, and console metadata.
- The controller accepts the device's self-signed TLS certificate (no certificate pinning).
- `adopt_viewport` is the primary entry point: given a device address and adoption parameters, it performs the POST and
  returns the result.
