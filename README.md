# Compute@Edge starter kit for Network Error Logging

A Rust based Compute@Edge starter kit for a [Network Error Logging](https://w3c.github.io/network-error-logging/) reporting endpoint.

**For more details about this and other starter kits for Compute@Edge, see the [Fastly developer hub](https://developer.fastly.com/solutions/starters)**

## What is network error logging?

From [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Network_Error_Logging):

> Network Error Logging is a mechanism that can be configured via the `NEL` HTTP response header. This experimental header allows web sites and applications to opt-in to receive reports about failed (and, if desired, successful) network fetches from supporting browsers.

Our starter kit will bootstrap a Fastly service that can receive network error reports, parse them, enrich them with information available at the edge, and dispatch them to your logging provider.

## Features

* Exposes a `POST /reports` endpoint to receive NEL reports
* Deserializes individual reports from JSON to Rust data structures
* Adds additional information at the edge, such as geo IP data
* Sends reports to a logging endpoint as individual JSON lines 
* Responds with a synthetic 204 response to the client

## Requirements

The following resources need to exist on your active Fastly service version for this starter kit to work:

- A [logging endpoint](https://docs.fastly.com/en/guides/about-fastlys-realtime-log-streaming-features) called `reports`.

## Security issues

Please see our [SECURITY.md](SECURITY.md) for guidance on reporting security-related issues.
