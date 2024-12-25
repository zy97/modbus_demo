use std::sync::LazyLock;

use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;

use opentelemetry::trace::{TraceContextExt, TraceError, Tracer};
use opentelemetry::{global, InstrumentationScope};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::logs::LogError;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::metrics::MetricError;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace as sdktrace;
use std::error::Error;
use tracing::info;
use tracing_subscriber::prelude::*;

pub static SERVICE_NAME: LazyLock<Resource> = LazyLock::new(|| {
    let service_name_resource = Resource::new(vec![KeyValue::new("service.name", "actix_server")]);
    service_name_resource
});
const OTLP_URL: &str = "http://10.39.10.126:4317";
pub fn init_traces() -> Result<sdktrace::TracerProvider, TraceError> {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(OTLP_URL)
        .build()?;
    Ok(sdktrace::TracerProvider::builder()
        .with_resource(SERVICE_NAME.clone())
        // .with_simple_exporter(exporter)
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build())
}
pub fn init_metrics() -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, MetricError> {
    let exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(OTLP_URL)
        .build()?;
    let reader = PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio).build();

    Ok(SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(SERVICE_NAME.clone())
        .build())
}
pub fn init_logs() -> Result<opentelemetry_sdk::logs::LoggerProvider, LogError> {
    let exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(OTLP_URL)
        .build()?;

    Ok(LoggerProvider::builder()
        .with_resource(SERVICE_NAME.clone())
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build())
}
