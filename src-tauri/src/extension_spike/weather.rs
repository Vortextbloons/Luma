use std::{sync::OnceLock, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";
const MAX_BODY: usize = 64 * 1024;
const MIN_TEMPERATURE: f64 = -150.0;
const MAX_TEMPERATURE: f64 = 150.0;
const MAX_WEATHER_CODE: u16 = 99;

static IN_FLIGHT: Mutex<()> = Mutex::const_new(());

struct Location {
    id: &'static str,
    latitude: f64,
    longitude: f64,
}

const LOCATIONS: [Location; 10] = [
    Location {
        id: "seattle",
        latitude: 47.6062,
        longitude: -122.3321,
    },
    Location {
        id: "los-angeles",
        latitude: 34.0522,
        longitude: -118.2437,
    },
    Location {
        id: "chicago",
        latitude: 41.8781,
        longitude: -87.6298,
    },
    Location {
        id: "new-york",
        latitude: 40.7128,
        longitude: -74.0060,
    },
    Location {
        id: "toronto",
        latitude: 43.6532,
        longitude: -79.3832,
    },
    Location {
        id: "london",
        latitude: 51.5074,
        longitude: -0.1278,
    },
    Location {
        id: "paris",
        latitude: 48.8566,
        longitude: 2.3522,
    },
    Location {
        id: "berlin",
        latitude: 52.5200,
        longitude: 13.4050,
    },
    Location {
        id: "tokyo",
        latitude: 35.6762,
        longitude: 139.6503,
    },
    Location {
        id: "sydney",
        latitude: -33.8688,
        longitude: 151.2093,
    },
];

pub(crate) struct Request {
    location: &'static Location,
    units: Units,
}

impl Request {
    pub(crate) fn new(location: &str, units: &str) -> Result<Self, Error> {
        let location = LOCATIONS
            .iter()
            .find(|candidate| candidate.id == location)
            .ok_or_else(Error::invalid)?;
        let units = Units::parse(units).ok_or_else(Error::invalid)?;
        Ok(Self { location, units })
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) enum Units {
    #[serde(rename = "c")]
    Celsius,
    #[serde(rename = "f")]
    Fahrenheit,
}

impl Units {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "c" => Some(Self::Celsius),
            "f" => Some(Self::Fahrenheit),
            _ => None,
        }
    }

    fn provider(self) -> &'static str {
        match self {
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Reading {
    temperature: f64,
    units: Units,
    weather_code: u16,
    observed_at: i64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Error {
    code: &'static str,
    message: &'static str,
}

impl Error {
    fn invalid() -> Self {
        Self {
            code: "invalid",
            message: "Choose a weather location in Settings.",
        }
    }

    fn busy() -> Self {
        Self {
            code: "busy",
            message: "Weather is still loading.",
        }
    }

    fn timeout() -> Self {
        Self {
            code: "timeout",
            message: "Weather took too long to load.",
        }
    }

    fn unavailable() -> Self {
        Self {
            code: "unavailable",
            message: "Weather is unavailable. Check your connection.",
        }
    }

    fn rejected() -> Self {
        Self {
            code: "unavailable",
            message: "Weather is unavailable. Try again.",
        }
    }

    fn too_large() -> Self {
        Self {
            code: "too_large",
            message: "Weather data was too large.",
        }
    }

    fn malformed() -> Self {
        Self {
            code: "malformed",
            message: "Weather data could not be read.",
        }
    }
}

#[derive(Deserialize)]
struct Payload {
    current: Option<Current>,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: Option<f64>,
    weather_code: Option<u16>,
    time: Option<i64>,
}

pub(crate) async fn read(request: Request) -> Result<Reading, Error> {
    let _guard = IN_FLIGHT.try_lock().map_err(|_| Error::busy())?;
    let response = client()?
        .get(forecast_url(request.location, request.units))
        .send()
        .await
        .map_err(transport)?;
    require_ok(response.status())?;
    let body = read_body(response).await?;
    project(&body, request.units)
}

fn client() -> Result<&'static reqwest::Client, Error> {
    static CLIENT: OnceLock<Result<reqwest::Client, Error>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .connect_timeout(Duration::from_secs(3))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| Error::unavailable())
        })
        .as_ref()
        .map_err(|error| *error)
}

fn forecast_url(location: &Location, units: Units) -> reqwest::Url {
    let mut url = reqwest::Url::parse(ENDPOINT).expect("valid weather endpoint");
    url.query_pairs_mut()
        .append_pair("latitude", &format!("{:.4}", location.latitude))
        .append_pair("longitude", &format!("{:.4}", location.longitude))
        .append_pair("current", "temperature_2m,weather_code")
        .append_pair("temperature_unit", units.provider())
        .append_pair("timeformat", "unixtime");
    url
}

async fn read_body(mut response: reqwest::Response) -> Result<Vec<u8>, Error> {
    if declared_too_large(response.content_length()) {
        return Err(Error::too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport)? {
        append_chunk(&mut body, &chunk)?;
    }
    Ok(body)
}

fn declared_too_large(length: Option<u64>) -> bool {
    length.is_some_and(|length| length > MAX_BODY as u64)
}

fn append_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), Error> {
    if body.len() + chunk.len() > MAX_BODY {
        return Err(Error::too_large());
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn require_ok(status: reqwest::StatusCode) -> Result<(), Error> {
    if status == reqwest::StatusCode::OK {
        Ok(())
    } else {
        Err(Error::rejected())
    }
}

fn transport(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::timeout()
    } else {
        Error::unavailable()
    }
}

fn project(body: &[u8], units: Units) -> Result<Reading, Error> {
    let payload = serde_json::from_slice::<Payload>(body).map_err(|_| Error::malformed())?;
    let current = payload.current.ok_or_else(Error::malformed)?;
    let temperature = checked_temperature(current.temperature_2m.ok_or_else(Error::malformed)?)?;
    let weather_code = current.weather_code.ok_or_else(Error::malformed)?;
    if weather_code > MAX_WEATHER_CODE {
        return Err(Error::malformed());
    }
    let observed_at = current.time.ok_or_else(Error::malformed)?;
    if observed_at <= 0 {
        return Err(Error::malformed());
    }
    Ok(Reading {
        temperature,
        units,
        weather_code,
        observed_at,
    })
}

fn checked_temperature(value: f64) -> Result<f64, Error> {
    if value.is_finite() && (MIN_TEMPERATURE..=MAX_TEMPERATURE).contains(&value) {
        Ok(value)
    } else {
        Err(Error::malformed())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        append_chunk, checked_temperature, declared_too_large, forecast_url, project, require_ok,
        Error, Location, Request, Units, ENDPOINT, LOCATIONS, MAX_BODY, MAX_TEMPERATURE,
        MAX_WEATHER_CODE, MIN_TEMPERATURE,
    };
    use serde_json::json;

    fn request(location: &str, units: &str) -> Result<Request, Error> {
        Request::new(location, units)
    }

    fn failure(body: &serde_json::Value, units: Units) -> Option<&'static str> {
        project(body.to_string().as_bytes(), units)
            .err()
            .map(|error| error.code)
    }

    fn payload() -> serde_json::Value {
        json!({
            "latitude": 47.625,
            "longitude": -122.375,
            "elevation": 63.0,
            "utc_offset_seconds": 0,
            "timezone": "GMT",
            "current_units": { "temperature_2m": "°F", "weather_code": "wmo code" },
            "current": {
                "time": 1_794_795_300_i64,
                "interval": 900,
                "temperature_2m": 46.0,
                "weather_code": 3
            }
        })
    }

    #[test]
    fn accepts_only_the_fixed_location_and_units_contract() {
        for location in [
            "seattle",
            "los-angeles",
            "chicago",
            "new-york",
            "toronto",
            "london",
            "paris",
            "berlin",
            "tokyo",
            "sydney",
        ] {
            assert!(request(location, "c").is_ok(), "{location} is a catalog id");
        }
        assert!(request("seattle", "f").is_ok());
        for location in [
            "",
            "atlantis",
            "Seattle",
            " seattle",
            "seattle ",
            "https://api.open-meteo.com/v1/forecast",
            "47.6062,-122.3321",
            "seattle&latitude=0",
            "seattle/../london",
            "seattle\n",
        ] {
            assert_eq!(
                request(location, "c").err().map(|error| error.code),
                Some("invalid"),
                "{location:?} must not reach the provider"
            );
        }
        for units in ["", "C", "F", "celsius", "fahrenheit", " c", "k", "1"] {
            assert_eq!(
                request("seattle", units).err().map(|error| error.code),
                Some("invalid"),
                "{units:?} must not reach the provider"
            );
        }
    }

    #[test]
    fn builds_the_fixed_provider_request() {
        let seattle = &LOCATIONS[0];
        for (units, temperature_unit) in [
            (Units::Celsius, "celsius"),
            (Units::Fahrenheit, "fahrenheit"),
        ] {
            let url = forecast_url(seattle, units);
            assert_eq!(url.scheme(), "https");
            assert_eq!(url.host_str(), Some("api.open-meteo.com"));
            assert_eq!(url.port(), None);
            assert_eq!(url.path(), "/v1/forecast");
            assert_eq!(url.username(), "");
            assert!(url.password().is_none());
            assert_eq!(url.as_str().split('?').next(), Some(ENDPOINT));
            let pairs: Vec<(String, String)> = url
                .query_pairs()
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect();
            let expected = [
                ("latitude", "47.6062"),
                ("longitude", "-122.3321"),
                ("current", "temperature_2m,weather_code"),
                ("temperature_unit", temperature_unit),
                ("timeformat", "unixtime"),
            ]
            .map(|(key, value)| (key.to_string(), value.to_string()));
            assert_eq!(pairs, expected.to_vec());
        }
    }

    #[test]
    fn projects_only_the_reading_contract_fields() {
        let reading = project(payload().to_string().as_bytes(), Units::Fahrenheit)
            .expect("a complete reading projects");
        assert_eq!(
            serde_json::to_value(&reading).expect("reading serializes"),
            json!({
                "temperature": 46.0,
                "units": "f",
                "weatherCode": 3,
                "observedAt": 1_794_795_300_i64
            })
        );
    }

    #[test]
    fn rejects_incomplete_or_implausible_current_conditions() {
        assert_eq!(failure(&payload(), Units::Celsius), None);
        assert_eq!(
            failure(
                &json!({ "current": { "temperature_2m": 150.0, "weather_code": MAX_WEATHER_CODE, "time": 1 } }),
                Units::Celsius
            ),
            None
        );
        for body in [
            json!({}),
            json!({ "current": null }),
            json!({ "current": { "temperature_2m": 46.0, "weather_code": 3 } }),
            json!({ "current": { "temperature_2m": 46.0, "weather_code": 3, "time": 0 } }),
            json!({ "current": { "temperature_2m": 46.0, "weather_code": 3, "time": -1 } }),
            json!({ "current": { "weather_code": 3, "time": 1 } }),
            json!({ "current": { "temperature_2m": null, "weather_code": 3, "time": 1 } }),
            json!({ "current": { "temperature_2m": "46", "weather_code": 3, "time": 1 } }),
            json!({ "current": { "temperature_2m": 46.0, "time": 1 } }),
            json!({ "current": { "temperature_2m": 46.0, "weather_code": 100, "time": 1 } }),
            json!({ "current": { "temperature_2m": 151.0, "weather_code": 3, "time": 1 } }),
            json!({ "current": { "temperature_2m": -150.5, "weather_code": 3, "time": 1 } }),
            json!({ "current": [] }),
            json!({ "current": "3" }),
        ] {
            assert_eq!(
                failure(&body, Units::Celsius),
                Some("malformed"),
                "{body} must not become a reading"
            );
        }
        assert_eq!(
            project(
                br#"{"current":{"temperature_2m":1e999,"weather_code":3,"time":1}}"#,
                Units::Celsius,
            )
            .err()
            .map(|error| error.code),
            Some("malformed")
        );
    }

    #[test]
    fn rejects_non_finite_or_out_of_range_temperatures() {
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            MAX_TEMPERATURE + 0.1,
            MIN_TEMPERATURE - 0.1,
        ] {
            assert_eq!(
                checked_temperature(value).err().map(|error| error.code),
                Some("malformed"),
                "{value} must not be displayed"
            );
        }
        assert_eq!(
            checked_temperature(MIN_TEMPERATURE).ok(),
            Some(MIN_TEMPERATURE)
        );
        assert_eq!(
            checked_temperature(MAX_TEMPERATURE).ok(),
            Some(MAX_TEMPERATURE)
        );
    }

    #[test]
    fn treats_every_non_success_status_as_unavailable_without_redirects() {
        assert!(require_ok(reqwest::StatusCode::OK).is_ok());
        for status in [301, 302, 304, 307, 308, 400, 401, 403, 404, 429, 500, 503] {
            let error = require_ok(reqwest::StatusCode::from_u16(status).expect("status"))
                .err()
                .expect("non-success status fails");
            assert_eq!(
                error.code, "unavailable",
                "HTTP {status} maps to unavailable"
            );
            assert!(!error.message.is_empty());
        }
    }

    #[test]
    fn caps_the_provider_body_before_and_during_reading() {
        assert!(!declared_too_large(None));
        assert!(!declared_too_large(Some(MAX_BODY as u64)));
        assert!(declared_too_large(Some(MAX_BODY as u64 + 1)));
        let mut body = Vec::new();
        append_chunk(&mut body, &vec![b' '; MAX_BODY]).expect("the exact cap is allowed");
        assert_eq!(body.len(), MAX_BODY);
        assert_eq!(
            append_chunk(&mut body, b" ").err().map(|error| error.code),
            Some("too_large")
        );
        assert_eq!(body.len(), MAX_BODY, "a rejected chunk is not appended");
    }

    #[test]
    fn the_reading_projection_keeps_location_out_of_the_payload() {
        let location = Location {
            id: "london",
            latitude: 51.5074,
            longitude: -0.1278,
        };
        let url = forecast_url(&location, Units::Celsius);
        assert_eq!(url.query_pairs().count(), 5);
        assert!(!url.as_str().contains("london"));
        let reading = project(payload().to_string().as_bytes(), Units::Celsius)
            .expect("a complete reading projects");
        let encoded = serde_json::to_string(&reading).expect("reading serializes");
        for private in [
            "latitude",
            "longitude",
            "timezone",
            "provider",
            "current",
            "interval",
        ] {
            assert!(
                !encoded.contains(private),
                "{private} must not leave the projection"
            );
        }
    }
}
