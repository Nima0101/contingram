//! Transparent opt-in event interface. This release has no collection endpoint.
//! No networking, identifiers, disk persistence, or arbitrary event properties.
use serde::Serialize;

/// Coarse command categories; never constructed from user-controlled text.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Analyze,
    Verify,
    Lower,
    Explain,
    Follow,
    Demo,
}
/// Coarse outcomes without diagnostics or model data.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    PolicyFound,
    NoPolicyWithinHorizon,
    Unknown,
    Success,
    InvalidInput,
    IoError,
    CheckError,
    InternalError,
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DurationBucket {
    Under10Ms,
    Under100Ms,
    Under1S,
    Under10S,
    AtLeast10S,
}
/// Closed event schema. No arbitrary maps, strings, model labels, paths or IDs.
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    schema_version: u32,
    version: &'static str,
    os_family: &'static str,
    architecture: &'static str,
    feature: Feature,
    outcome: Outcome,
    duration: DurationBucket,
}
impl Event {
    pub fn new(feature: Feature, outcome: Outcome, duration: std::time::Duration) -> Self {
        let duration = match duration.as_millis() {
            0..=9 => DurationBucket::Under10Ms,
            10..=99 => DurationBucket::Under100Ms,
            100..=999 => DurationBucket::Under1S,
            1000..=9999 => DurationBucket::Under10S,
            _ => DurationBucket::AtLeast10S,
        };
        Self {
            schema_version: 1,
            version: env!("CARGO_PKG_VERSION"),
            os_family: match std::env::consts::OS {
                "linux" => "linux",
                "macos" => "macos",
                "windows" => "windows",
                _ => "other",
            },
            architecture: match std::env::consts::ARCH {
                "x86_64" => "x86_64",
                "aarch64" => "aarch64",
                _ => "other",
            },
            feature,
            outcome,
            duration,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct TelemetryStatus {
    pub opt_in_requested: bool,
    pub do_not_track: bool,
    pub collection_enabled: bool,
    pub endpoint: Option<&'static str>,
    pub operator: Option<&'static str>,
    pub retention: &'static str,
    pub reason: &'static str,
}
/// Resolve explicit consent. `0`, absent or invalid settings disable; DNT wins.
/// Only exact `CONTINGRAM_TELEMETRY=1` requests opt-in. Any nonempty DNT value
/// other than `0` conservatively vetoes collection.
pub fn status_from(
    telemetry: Option<&str>,
    do_not_track: Option<&str>,
    dnt: Option<&str>,
) -> TelemetryStatus {
    let veto = [do_not_track, dnt]
        .into_iter()
        .flatten()
        .any(|v| !v.is_empty() && v != "0");
    let requested = telemetry == Some("1");
    TelemetryStatus {
        opt_in_requested: requested,
        do_not_track: veto,
        collection_enabled: false,
        endpoint: None,
        operator: None,
        retention: "none: no data collected or retained",
        reason: if veto {
            "do_not_track"
        } else if !requested {
            "disabled"
        } else {
            "no_first_party_endpoint"
        },
    }
}
/// Read only the three documented opt-in/kill-switch environment variables.
pub fn status() -> TelemetryStatus {
    status_from(
        std::env::var("CONTINGRAM_TELEMETRY").ok().as_deref(),
        std::env::var("DO_NOT_TRACK").ok().as_deref(),
        std::env::var("DNT").ok().as_deref(),
    )
}
/// Bounded nonblocking submission interface. Returns false in this release:
/// there is no approved first-party endpoint, queue, transport or disk spool.
/// Future transports must be asynchronous, bounded and failure-isolated.
pub fn try_emit(_event: &Event) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consent_and_kill_switch_never_send_without_endpoint() {
        for t in [
            None,
            Some("0"),
            Some("1"),
            Some("true"),
            Some("https://unrelated.invalid"),
        ] {
            for d in [None, Some("0"), Some("1"), Some("yes")] {
                let s = status_from(t, d, None);
                assert!(!s.collection_enabled);
                assert!(s.endpoint.is_none());
                assert!(!try_emit(&Event::new(
                    Feature::Analyze,
                    Outcome::Unknown,
                    std::time::Duration::ZERO
                )));
            }
        }
        assert_eq!(
            status_from(Some("1"), None, Some("1")).reason,
            "do_not_track"
        );
        assert_eq!(status_from(Some("0"), None, None).reason, "disabled");
        assert_eq!(
            status_from(Some("1"), None, None).reason,
            "no_first_party_endpoint"
        );
    }
    #[test]
    fn opt_in_event_schema_is_closed_and_bounded() {
        let event = Event::new(
            Feature::Verify,
            Outcome::CheckError,
            std::time::Duration::from_secs(100),
        );
        let v = serde_json::to_value(&event).unwrap();
        let mut keys: Vec<_> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "architecture",
                "duration",
                "feature",
                "os_family",
                "outcome",
                "schema_version",
                "version"
            ]
        );
        assert_eq!(v["duration"], "at_least10_s");
        assert!(serde_json::to_vec(&event).unwrap().len() < 512);
    }
}
