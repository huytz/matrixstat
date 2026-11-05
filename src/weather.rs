//! Local weather: IP location (ipinfo.io) + Open-Meteo, on a background thread.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Open-Meteo updates current conditions every 15 minutes.
const REFRESH: Duration = Duration::from_secs(15 * 60);
/// Retry sooner after a failure (offline, captive portal, rate limit).
const RETRY: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct Weather {
    pub place: String,
    pub temp: f64,
    pub feels: f64,
    pub humidity: f64,
    pub wind_kmh: f64,
    pub hi: f64,
    pub lo: f64,
    pub code: u8,
    pub is_day: bool,
    pub at: Instant,
}

impl Weather {
    /// Text-presentation glyph (U+FE0E) for `--once` output.
    pub fn icon(&self) -> &'static str {
        match self.code {
            0 | 1 if self.is_day => "☀\u{FE0E}",
            0 | 1 => "☾\u{FE0E}",
            2 => "⛅\u{FE0E}",
            3 => "☁\u{FE0E}",
            45 | 48 => "≋",
            51..=67 | 80..=82 => "☂\u{FE0E}",
            71..=77 | 85 | 86 => "❄\u{FE0E}",
            95..=99 => "⚡\u{FE0E}",
            _ => "☁\u{FE0E}",
        }
    }

    /// SF Symbol span ("sf:<name>") for the menu bar and dropdown header.
    pub fn symbol(&self) -> &'static str {
        match self.code {
            0 | 1 if self.is_day => "sf:sun.max.fill",
            0 | 1 => "sf:moon.stars.fill",
            2 if self.is_day => "sf:cloud.sun.fill",
            2 => "sf:cloud.moon.fill",
            3 => "sf:cloud.fill",
            45 | 48 => "sf:cloud.fog.fill",
            51..=57 => "sf:cloud.drizzle.fill",
            61..=67 | 80..=82 => "sf:cloud.rain.fill",
            71..=77 | 85 | 86 => "sf:cloud.snow.fill",
            95..=99 => "sf:cloud.bolt.rain.fill",
            _ => "sf:cloud.fill",
        }
    }

    /// SF Symbol span for the unit after the temperature.
    pub const CELSIUS: &str = "sf:degreesign.celsius";

    /// WMO weather interpretation code as a short label.
    pub fn label(&self) -> &'static str {
        match self.code {
            0 => "Clear",
            1 => "Mostly clear",
            2 => "Partly cloudy",
            3 => "Overcast",
            45 | 48 => "Fog",
            51..=57 => "Drizzle",
            61..=67 => "Rain",
            71..=77 => "Snow",
            80..=82 => "Showers",
            85 | 86 => "Snow showers",
            95..=99 => "Thunderstorm",
            _ => "Unknown",
        }
    }
}

pub type Shared = Arc<Mutex<Option<Weather>>>;

/// Start the fetch loop; the returned slot holds the latest good reading.
pub fn spawn() -> Shared {
    let slot: Shared = Arc::new(Mutex::new(None));
    let out = slot.clone();
    std::thread::spawn(move || {
        loop {
            let wait = match fetch() {
                Some(w) => {
                    *out.lock().unwrap_or_else(|e| e.into_inner()) = Some(w);
                    REFRESH
                }
                None => RETRY,
            };
            std::thread::sleep(wait);
        }
    });
    slot
}

/// One blocking round trip: locate, then read current weather.
pub fn fetch() -> Option<Weather> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let (lat, lon, place) = locate(&agent)?;
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}\
         &current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,\
         wind_speed_10m,is_day&daily=temperature_2m_max,temperature_2m_min\
         &timezone=auto&forecast_days=1"
    );
    let v = get_json(&agent, &url)?;
    parse_forecast(&v, place)
}

/// MATRIXSTAT_LOCATION="lat,lon" overrides IP lookup (VPN exits can be far off).
fn locate(agent: &ureq::Agent) -> Option<(f64, f64, String)> {
    if let Some((lat, lon)) = std::env::var("MATRIXSTAT_LOCATION")
        .ok()
        .as_deref()
        .and_then(parse_latlon)
    {
        return Some((lat, lon, format!("{lat:.2}, {lon:.2}")));
    }
    let v = get_json(agent, "https://ipinfo.io/json")?;
    let (lat, lon) = parse_latlon(v["loc"].as_str()?)?;
    let place = [v["city"].as_str(), v["country"].as_str()]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    Some((lat, lon, place))
}

fn get_json(agent: &ureq::Agent, url: &str) -> Option<Value> {
    let body = agent
        .get(url)
        .header(
            "User-Agent",
            concat!("matrixstat/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    serde_json::from_str(&body).ok()
}

fn parse_latlon(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn parse_forecast(v: &Value, place: String) -> Option<Weather> {
    let c = &v["current"];
    let d = &v["daily"];
    Some(Weather {
        place,
        temp: c["temperature_2m"].as_f64()?,
        feels: c["apparent_temperature"].as_f64().unwrap_or(f64::NAN),
        humidity: c["relative_humidity_2m"].as_f64().unwrap_or(f64::NAN),
        wind_kmh: c["wind_speed_10m"].as_f64().unwrap_or(f64::NAN),
        hi: d["temperature_2m_max"][0].as_f64().unwrap_or(f64::NAN),
        lo: d["temperature_2m_min"][0].as_f64().unwrap_or(f64::NAN),
        code: c["weather_code"].as_u64().unwrap_or(255) as u8,
        is_day: c["is_day"].as_u64() != Some(0),
        at: Instant::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latlon() {
        assert_eq!(parse_latlon("10.82,106.63"), Some((10.82, 106.63)));
        assert_eq!(parse_latlon(" 1.3 , 103.8 "), Some((1.3, 103.8)));
        assert_eq!(parse_latlon("nope"), None);
    }

    #[test]
    fn forecast_and_icons() {
        let v: Value = serde_json::from_str(
            r#"{"current":{"temperature_2m":31.4,"apparent_temperature":36.0,
            "relative_humidity_2m":70,"weather_code":61,"wind_speed_10m":9.5,"is_day":1},
            "daily":{"temperature_2m_max":[33.1],"temperature_2m_min":[25.2]}}"#,
        )
        .unwrap();
        let mut w = parse_forecast(&v, "Ho Chi Minh City, VN".into()).unwrap();
        assert_eq!(w.label(), "Rain");
        assert_eq!(w.icon(), "☂\u{FE0E}");
        assert_eq!(w.symbol(), "sf:cloud.rain.fill");
        assert_eq!(w.hi, 33.1);
        w.code = 0;
        w.is_day = false;
        assert_eq!(w.icon(), "☾\u{FE0E}");
    }
}
