# Compute@Edge starter kit for Network Error Logging

A Rust based Compute@Edge stater kit for a [Network Error Logging][spec] reporting endpoint.

> Network Error Logging is a mechanism that can be configured via the NEL HTTP response header. This experimental header allows web sites and applications to opt-in to receive reports about failed (and, if desired, successful) network fetches from supporting browsers.
(source: [MDN][MDN])

**For more details about this and other starter kits for Compute@Edge, see the [Fastly developer hub](https://developer.fastly.com/solutions/starters)**

[spec]: https://w3c.github.io/network-error-logging/
[MDN]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Network_Error_Logging

## Usage
- Install the [Fastly CLI][latest] to your `$PATH`
- Configure the CLI with an API token via `fastly configure`
- Run `fastly compute init --from https://github.com/fastly/fastly-template-rust-nel.git`

[cli]: https://github.com/fastly/cli
[latest]: https://github.com/fastly/cli/releases/latest

## Features

* Exposes a `POST /reports` endpoint to receive NEL reports
* Deserializes individual reports from JSON to Rust data structures
* Adds additional information at the edge such as geo IP data
* Sends reports to a logging endpoint as individual JSON lines 
* Responds with a synthetic 204 response from the edge

## Requirements:
The following resources need to exist on your active service version for the 
service to work end-to-end.

- A [logging endpoint][logging] called `reports`.

[logging]: https://docs.fastly.com/en/guides/about-fastlys-realtime-log-streaming-features

## Security issues

Please see our [SECURITY.md](SECURITY.md) for guidance on reporting security-related issues.
