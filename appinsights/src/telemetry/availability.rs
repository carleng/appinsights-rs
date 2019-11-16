use std::time::Duration as StdDuration;

use chrono::{DateTime, SecondsFormat, Utc};

use crate::context::TelemetryContext;
use crate::contracts::*;
use crate::telemetry::{ContextTags, Measurements, Properties, Telemetry};
use crate::time::{self, Duration};
use crate::uuid::Uuid;

/// Represents the result of executing an availability test.
pub struct AvailabilityTelemetry {
    /// Identifier of a test run.
    /// It is used to correlate steps of test run and telemetry generated by the service.
    id: Option<Uuid>,

    /// Name of the test that this result represents.
    name: String,

    /// Duration of the test run.
    duration: Duration,

    /// Indication of successful or unsuccessful call.
    success: bool,

    /// The time stamp when this telemetry was measured.
    timestamp: DateTime<Utc>,

    /// Name of the location where the test was run.
    run_location: Option<String>,

    /// Diagnostic message for the result.
    message: Option<String>,

    /// Custom properties.
    properties: Properties,

    /// Telemetry context containing extra, optional tags.
    tags: ContextTags,

    /// Custom measurements.
    measurements: Measurements,
}

impl AvailabilityTelemetry {
    /// Creates a new availability telemetry item with the specified test name, duration and success code.
    pub fn new(name: String, duration: StdDuration, success: bool) -> Self {
        Self {
            id: Default::default(),
            name,
            duration: duration.into(),
            run_location: Default::default(),
            message: Default::default(),
            success,
            timestamp: time::now(),
            properties: Default::default(),
            tags: Default::default(),
            measurements: Default::default(),
        }
    }

    /// Returns custom measurements to submit with the telemetry item.
    pub fn measurements(&self) -> &Measurements {
        &self.measurements
    }

    /// Returns mutable reference to custom measurements.
    pub fn measurements_mut(&mut self) -> &mut Measurements {
        &mut self.measurements
    }
}

impl Telemetry for AvailabilityTelemetry {
    /// Returns the time when this telemetry was measured.
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Returns custom properties to submit with the telemetry item.
    fn properties(&self) -> &Properties {
        &self.properties
    }

    /// Returns mutable reference to custom properties.
    fn properties_mut(&mut self) -> &mut Properties {
        &mut self.properties
    }

    /// Returns context data containing extra, optional tags. Overrides values found on client telemetry context.
    fn tags(&self) -> &ContextTags {
        &self.tags
    }

    /// Returns mutable reference to custom tags.
    fn tags_mut(&mut self) -> &mut ContextTags {
        &mut self.tags
    }
}

impl From<(TelemetryContext, AvailabilityTelemetry)> for Envelope {
    fn from((context, telemetry): (TelemetryContext, AvailabilityTelemetry)) -> Self {
        let data = Data::AvailabilityData({
            let id = telemetry
                .id
                .map(|id| id.to_hyphenated().to_string())
                .unwrap_or_default();
            let mut builder =
                AvailabilityDataBuilder::new(id, telemetry.name, telemetry.duration.to_string(), telemetry.success);
            builder
                .properties(Properties::combine(context.properties, telemetry.properties))
                .measurements(telemetry.measurements);

            if let Some(run_location) = telemetry.run_location {
                builder.run_location(run_location);
            }

            if let Some(message) = telemetry.message {
                builder.run_location(message);
            }

            builder.build()
        });

        let envelope_name = data.envelope_name(&context.normalized_i_key);
        let timestamp = telemetry.timestamp.to_rfc3339_opts(SecondsFormat::Millis, true);

        EnvelopeBuilder::new(envelope_name, timestamp)
            .data(Base::Data(data))
            .i_key(context.i_key)
            .tags(ContextTags::combine(context.tags, telemetry.tags))
            .build()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::TimeZone;

    use super::*;

    #[test]
    fn it_overrides_properties_from_context() {
        time::set(Utc.ymd(2019, 1, 2).and_hms_milli(3, 4, 5, 800));

        let mut context = TelemetryContext::new("instrumentation".into());
        context.properties_mut().insert("test".into(), "ok".into());
        context.properties_mut().insert("no-write".into(), "fail".into());

        let mut telemetry = AvailabilityTelemetry::new(
            "GET https://example.com/main.html".into(),
            StdDuration::from_secs(2),
            true,
        );
        telemetry.properties_mut().insert("no-write".into(), "ok".into());
        telemetry.measurements_mut().insert("latency".into(), 200.0);

        let envelop = Envelope::from((context, telemetry));

        let expected = EnvelopeBuilder::new(
            "Microsoft.ApplicationInsights.instrumentation.Availability",
            "2019-01-02T03:04:05.800Z",
        )
        .data(Base::Data(Data::AvailabilityData(
            AvailabilityDataBuilder::new("", "GET https://example.com/main.html", "0.00:00:02.0000000", true)
                .properties({
                    let mut properties = BTreeMap::default();
                    properties.insert("test".into(), "ok".into());
                    properties.insert("no-write".into(), "ok".into());
                    properties
                })
                .measurements({
                    let mut measurement = Measurements::default();
                    measurement.insert("latency".into(), 200.0);
                    measurement
                })
                .build(),
        )))
        .i_key("instrumentation")
        .tags(BTreeMap::default())
        .build();

        assert_eq!(envelop, expected)
    }

    #[test]
    fn it_overrides_tags_from_context() {
        time::set(Utc.ymd(2019, 1, 2).and_hms_milli(3, 4, 5, 700));

        let mut context = TelemetryContext::new("instrumentation".into());
        context.tags_mut().insert("test".into(), "ok".into());
        context.tags_mut().insert("no-write".into(), "fail".into());

        let mut telemetry = AvailabilityTelemetry::new(
            "GET https://example.com/main.html".into(),
            StdDuration::from_secs(2),
            true,
        );
        telemetry.measurements_mut().insert("latency".into(), 200.0);
        telemetry.tags_mut().insert("no-write".into(), "ok".into());

        let envelop = Envelope::from((context, telemetry));

        let expected = EnvelopeBuilder::new(
            "Microsoft.ApplicationInsights.instrumentation.Availability",
            "2019-01-02T03:04:05.700Z",
        )
        .data(Base::Data(Data::AvailabilityData(
            AvailabilityDataBuilder::new("", "GET https://example.com/main.html", "0.00:00:02.0000000", true)
                .properties(Properties::default())
                .measurements({
                    let mut measurement = Measurements::default();
                    measurement.insert("latency".into(), 200.0);
                    measurement
                })
                .build(),
        )))
        .i_key("instrumentation")
        .tags({
            let mut tags = BTreeMap::default();
            tags.insert("test".into(), "ok".into());
            tags.insert("no-write".into(), "ok".into());
            tags
        })
        .build();

        assert_eq!(envelop, expected)
    }
}
