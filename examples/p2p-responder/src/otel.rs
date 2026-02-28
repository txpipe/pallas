use opentelemetry::global;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{
    LogExporter, MetricExporter, Protocol, SpanExporter, WithExportConfig as _,
};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::sync::OnceLock;
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer as _};

fn get_resource() -> Resource {
    static RESOURCE: OnceLock<Resource> = OnceLock::new();
    RESOURCE
        .get_or_init(|| Resource::builder().with_service_name("pallas-p2p-responder").build())
        .clone()
}

fn init_traces() -> SdkTracerProvider {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_protocol(Protocol::Grpc)
        .build()
        .expect("Failed to create span exporter");

    SdkTracerProvider::builder()
        .with_resource(get_resource())
        .with_batch_exporter(exporter)
        .build()
}

fn init_metrics() -> SdkMeterProvider {
    let exporter = MetricExporter::builder()
        .with_tonic()
        .with_protocol(Protocol::Grpc)
        .build()
        .expect("Failed to create metric exporter");

    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(1))
        .build();

    SdkMeterProvider::builder()
        .with_resource(get_resource())
        .with_reader(reader)
        .build()
}

fn init_logs() -> SdkLoggerProvider {
    let exporter = LogExporter::builder()
        .with_tonic()
        .with_protocol(Protocol::Grpc)
        .build()
        .expect("Failed to create log exporter");

    SdkLoggerProvider::builder()
        .with_resource(get_resource())
        .with_batch_exporter(exporter)
        .build()
}

pub fn setup_otel() {
    let logger_provider = init_logs();
    let otel_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    let filter_otel = EnvFilter::new("info")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());

    let otel_layer = otel_layer.with_filter(filter_otel);

    let filter_fmt = EnvFilter::new("info")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap())
        .add_directive("tower=off".parse().unwrap())
        .add_directive("opentelemetry=off".parse().unwrap());

    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(filter_fmt);

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();

    let tracer_provider = init_traces();
    global::set_tracer_provider(tracer_provider.clone());

    let meter_provider = init_metrics();
    global::set_meter_provider(meter_provider.clone());
}
