//! # Network Error Logging
//!
//! A Compute@Edge service which exposes a HTTP reporting endpoint for the
//! W3C [Network Error Logging API](https://www.w3.org/TR/network-error-logging).
use chrono::Utc;
use fastly::error::anyhow;
use fastly::http::{header, HeaderValue, Method, StatusCode};
use fastly::log::Endpoint;
use fastly::{downstream_client_ip_addr, uap_parse, Body, Error, Request, Response, ResponseExt};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Write;
use std::net::IpAddr;
use std::str::FromStr;

/// ReportBody models the body of a Network Error Log report which details the
/// network error that occurred.
///
/// Note: view the Network Error Logging [specification](https://www.w3.org/TR/network-error-logging]
/// for detailed information on the report structure.
#[derive(Serialize, Deserialize, Clone)]
pub struct ReportBody {
    #[serde(rename = "type")]
    pub error_type: String,
    pub elapsed_time: i32,
    pub method: String,
    pub phase: String,
    pub protocol: String,
    pub referrer: String,
    pub sampling_fraction: f32,
    pub server_ip: String,
    pub status_code: i32,
}

/// Report models a NEL report, a collection of arbitrary data which a user agent
/// is expected to deliver to the report endpoint.
///
/// Each report has a `body`, which is either null or an object which can be
/// serialized into a JSON text. The fields contained in a report’s body are
/// determined by the report’s type.
///
/// Each report has a `url`, which is typically the address of the Document or
/// Worker from which the report was generated.
///
/// Each report has a `user_agent`, which is the value of the User-Agent header
/// of the request from which the report was generated.
///
/// Each report has a `type`, which is a `ReportType`.
#[derive(Serialize, Deserialize, Clone)]
pub struct Report {
    pub user_agent: String,
    pub url: String,
    #[serde(rename = "type")]
    pub report_type: String,
    pub body: ReportBody,
    pub age: i64,
}

/// Parses an IP string input and truncates it to a privacy safe prefix mask and
/// returns the network as a CIDR string, such as `167.98.105.176/28`.
///
/// For IPv4 addresses we truncate to a /28 prefix and for IPv6 addresses we
/// truncate to /56.
pub fn truncate_ip_to_prefix(ip: IpAddr) -> Result<String, Error> {
    match ip {
        IpAddr::V4(ip) => ipnet::Ipv4Net::new(ip, 28)
            .map(|i| i.trunc().to_string())
            .map_err(Error::new),
        IpAddr::V6(ip) => ipnet::Ipv6Net::new(ip, 56)
            .map(|i| i.trunc().to_string())
            .map_err(Error::new),
    }
}

/// UserAgent is a structured representation of a User Agent string which lets
/// servers and network peers identify the application, operating system,
/// vendor, and/or version of the requesting user agent.
///
/// Implements the `FromStr` trait to facilitate parsing from a User-Agent header
/// value.
#[derive(Clone)]
pub struct UserAgent {
    family: String,
    major: String,
    minor: String,
    patch: String,
}

impl FromStr for UserAgent {
    type Err = Error;

    fn from_str(s: &str) -> Result<UserAgent, Error> {
        let (family, major, minor, patch) = uap_parse(s)?;
        Ok(UserAgent {
            family,
            major: major.unwrap_or_default(),
            minor: minor.unwrap_or_default(),
            patch: patch.unwrap_or_default(),
        })
    }
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}.{}.{}",
            self.family, self.major, self.minor, self.patch
        )
    }
}

/// ClientData models information about the client which sent the NEL report
/// request, such as geo IP data and User Agent.
#[derive(Serialize, Deserialize, Clone)]
pub struct ClientData {
    client_ip: String,
    client_user_agent: String,
    client_asn: u32,
    client_asname: String,
    client_city: String,
    client_region: String,
    client_country_code: String,
    client_continent_code: fastly::geo::Continent,
    client_latitude: f64,
    client_longitude: f64,
}

impl ClientData {
    /// Returns a `ClientData` using information from the downstream request.
    pub fn new(client_ip: IpAddr, client_user_agent: &str) -> Result<ClientData, Error> {
        // Lookup the geo IP data from the client IP.
        match fastly::geo::geo_lookup(client_ip) {
            Some(geo) => Ok(ClientData {
                client_ip: truncate_ip_to_prefix(client_ip)?, // Truncate the IP to a privacy safe prefix.
                client_user_agent: UserAgent::from_str(client_user_agent)?.to_string(), // Parse the User-Agent string to family, major, minor, patch.
                client_asn: geo.as_number(),
                client_asname: geo.as_name().to_string(),
                client_city: geo.city().to_string(),
                client_region: geo.region().unwrap_or("").to_string(),
                client_country_code: geo.country_code().to_string(),
                client_latitude: geo.latitude(),
                client_longitude: geo.longitude(),
                client_continent_code: geo.continent(),
            }),
            None => Err(anyhow!("Unable to lookup geo IP data")),
        }
    }
}

/// LogLine models the structure of the log line we want to serialize to JSON
/// and emit to the logging endpoint.
///
/// Consists of:
/// - `timestamp`, a unix timestamp generated when we receive the log.
/// - `client`, a `ClientData` object.
/// - `report`, a `Report` which has been sanitized.
#[derive(Serialize, Deserialize)]
pub struct LogLine {
    timestamp: i64,
    client: ClientData,
    report: Report,
}

impl LogLine {
    // Construct a new LogLine from a `Report` and `ClientData` and decorate
    // with a Unix timestamp.
    pub fn new(report: Report, client: ClientData) -> Result<LogLine, Error> {
        Ok(LogLine {
            timestamp: Utc::now().timestamp(),
            client,
            report,
        })
    }
}

/// Utility to generate a synthetic `204 No Content` response.
///
/// Generates a response with a 204 status code, no-cache cache-control and
/// appropriate CORS headers required for the NEL request.
pub fn generate_no_content_response() -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CACHE_CONTROL,
            "no-cache, no-store, max-age=0, must-revalidate",
        )
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCESS_CONTROL_ALLOW_HEADERS, header::CONTENT_TYPE)
        .header(header::ACCESS_CONTROL_ALLOW_METHODS, "POST, OPTIONS")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from("No content"))?)
}

// Utility function to get a header value as a string. Returns an empty string
// if no value is found.
fn header_val(header: Option<&HeaderValue>) -> &str {
    match header {
        Some(h) => h.to_str().unwrap_or(""),
        None => "",
    }
}

/// Handle reports
///
/// It attempts to extract the NEL reports from the POST request body and maps
/// over each report adding additional information before emitting a log line
/// to the `reports` logging endpoint if valid. It always returns a synthetic
/// `204 No content` response, regardless of whether the log reporting was
/// successful.
fn handle_reports(req: Request<Body>) -> Result<Response<Body>, Error> {
    let (parts, body) = req.into_parts();

    // Parse the NEL reports from the request JSON body using serde_json.
    // If successful, bind the reports to the `reports` variable, transform and log.
    if let Ok(reports) = serde_json::from_reader::<Body, Vec<Report>>(body) {
        // Extract information about the client from the downstream request,
        // such as the User-Agent and IP address.
        let client_user_agent = header_val(parts.headers.get(header::USER_AGENT));
        let client_ip = downstream_client_ip_addr().expect("should have client IP");

        // Construct a new `ClientData` structure from the User-Agent and IP.
        let client_data = ClientData::new(client_ip, client_user_agent)?;

        // Map over each raw NEL report, merge it with the `ClientData` from
        // above and transform it to a `LogLine` structure to merging it with the ClientData
        let logs: Vec<LogLine> = reports
            .into_iter()
            .map(|report| LogLine::new(report, client_data.clone()))
            .filter_map(Result::ok)
            .collect();

        // Create a handle to the upstream logging endpoint that we want to emit
        // the reports too.
        let mut endpoint = Endpoint::from_name("reports");

        // Loop over each log line serializing it back to JSON and write it to
        // the logging endpoint.
        for log in logs.iter() {
            if let Ok(json) = serde_json::to_string(&log) {
                writeln!(endpoint, "{}", json)?;
            }
        }
    };

    // Return and empty 204 no content response to the downstream client,
    // regardless of successful logging.
    generate_no_content_response()
}

/// Main application entrypoint.
///
/// This controls the routing logic for the application, it accepts a `Request`
/// and passes it to any matching request handlers before returning a `Response`
/// back downstream.
#[fastly::main]
fn main(req: Request<Body>) -> Result<impl ResponseExt, Error> {
    // Pattern match on the request method and path.
    match (req.method(), req.uri().path()) {
        // If a CORs preflight OPTIONS request return a 204 no content.
        (&Method::OPTIONS, "/report") => generate_no_content_response(),
        // If a POST request pass to the `handler_reports` request handler.
        (&Method::POST, "/report") => handle_reports(req),
        // For all other requests return a 404 not found.
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))?),
    }
}
