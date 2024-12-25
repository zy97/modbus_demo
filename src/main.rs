mod app_config;
mod modbus_manager;
mod otlp;
mod server_router;
use actix_web::{middleware, web, App, HttpServer};
use actix_web_opentelemetry::{PrometheusMetricsHandler, RequestMetrics, RequestTracing};
use app_config::{load_config, AppConfig};
use modbus_manager::{ModbusContext, Pool};
use opentelemetry::InstrumentationScope;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::{self, OpenTelemetryTracingBridge};
use opentelemetry_otlp::{TonicExporterBuilder, WithExportConfig};
use opentelemetry_sdk::Resource;
use otlp::init_logs;
use otlp::init_metrics;
use otlp::init_traces;
use server_router::{get_modbus_value, greet};
use std::{collections::HashMap, sync::LazyLock};
use tracing::instrument::WithSubscriber;
use tracing::{debug, info};
use tracing_opentelemetry::layer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, FmtSubscriber};
static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    // M3 Ultra takes about 16 million years in --release config
    let config = load_config().unwrap();
    debug!("加载配置成功：{:#?}", config);
    config
});

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let logger_provider = init_logs().unwrap();
    let otel_layer = OpenTelemetryTracingBridge::new(&logger_provider);
    let filter_otel = EnvFilter::new("info")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("opentelemetry=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());
    let otel_layer = otel_layer.with_filter(filter_otel);

    let filter_fmt = EnvFilter::new("info").add_directive("opentelemetry=debug".parse().unwrap());
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_filter(filter_fmt);

    let tracer_provider = init_traces().unwrap();
    global::set_tracer_provider(tracer_provider.clone());
    let meter_provider = init_metrics().unwrap();
    global::set_meter_provider(meter_provider.clone());
    let common_scope_attributes = vec![KeyValue::new("scope-key", "scope-value")];
    let scope = InstrumentationScope::builder("basic")
        .with_version("1.0")
        .with_attributes(common_scope_attributes)
        .build();

    let tracer = global::tracer_with_scope(scope.clone());
    let meter = global::meter_with_scope(scope);

    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // 创建一个文件输出层
    let file_layer = Layer::new().with_writer(non_blocking); // 输出到文件

    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish()
        .with(file_layer);

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        // .with_subscriber(subscriber)
        .init()
        .with_subscriber(subscriber)
        .dispatcher();
    // tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    info!("1");

    let pools: HashMap<String, Pool> = APP_CONFIG
        .modbus
        .configs
        .iter()
        .map(|config| {
            let mgr = ModbusContext {
                addr: config.address.to_string(),
                slave: config.slave_id,
            };
            let modbus_pool = Pool::builder(mgr).max_size(50).build().unwrap();
            (config.name.clone(), modbus_pool)
        })
        .collect();
    let server_url = &*APP_CONFIG.server.address;
    info!("1");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pools.clone()))
            .service(greet)
            .service(get_modbus_value)
            .wrap(middleware::Logger::default())
    })
    .bind(server_url)?
    .run()
    .await?;
    tracer_provider.shutdown().unwrap();
    meter_provider.shutdown().unwrap();
    logger_provider.shutdown().unwrap();
    Ok(())
}
