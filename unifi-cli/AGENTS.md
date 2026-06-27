# unifi-cli

A simple CLI app that wraps the SDKs in this repo. The CLI targets modern versions of Windows, macOS, and Linux.

Purpose:

- To help in manually testing implementations and SDK against real hardware.
- To aid in debugging implementations that use the SDKs.
- To provide a simple command-line interface for end-users for simple tasks such as discovering devices.

## Flag Conventions

- Only CLI flags that would be useful for end-users are should be included.
- The CLI should be designed with simplicity and ease of use in mind, with a focus on providing a straightforward
  interface for end-users to interact with the SDKs. Provide reasonable defaults when possible.
- Any destructive actions should be clearly indicated and require a `--force` flag to execute.
