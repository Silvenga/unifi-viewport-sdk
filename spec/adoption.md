# Adoption Protocol

> Captures are a bit harder here, since the device talks over TLS 1.3. Some trial and error was required here.

After being discovered, the device is shown in the UI, ready for adoption.

```bash
curl -kv -X POST \
  -H 'Content-Type: application/json' \
  -d '{"username":"ui","password":"ui","hosts":["192.168.0.4:7442"],"token":"test","protocol":"wss","mode":0,"nvr":"UNVR4","controller":"Protect","consoleId":"test","consoleName":"test"}' \
  https://192.168.0.201:8080/api/adopt
```

The device's internal server (on port `8080`) talks over TLS 1.3 using a server certificate generated on factory reset.

After this adoption request, the device returns "Successful" and the device attempts to connect to the controller using
the `ucp4` protocol over `wss` using a client certificate (also generated on factory reset).
