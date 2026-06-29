# Purposeful Deviations from Spec

This file documents intentional deviations from the protocol spec in `spec/`. These are
deliberate design decisions, not bugs.

- TLS 1.2 is not supported.
- Certificate subject is `CN=unifi-device-viewport` instead of the spec's `CN=UI RSA, O=UI`.
- Settings handler only returns 200 OK. The spec's `device.adb`, `device.volume`,  `device.auth`, `wifi`, and `net`
  sub-objects were added to the firmware to test a theory and are no longer needed. The endpoint still accepts and 200s
  the POST request (matching the observed controller behavior of repeatedly hitting the route), but does not process
  those sub-objects.
