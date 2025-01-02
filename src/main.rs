mod app_config;
mod modbus_manager;
mod otlp;
mod server_router;
mod trace_middleware;
use actix_web::dev::Service;
use actix_web::middleware::from_fn;
use actix_web::{middleware, web, App, HttpServer};
use app_config::{load_config, AppConfig};
use futures_util::FutureExt as _;
use modbus_manager::{ModbusManager, Pool};
use opentelemetry::global;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use otlp::{init_logs, init_traces};
use server_router::{get_modbus_value, greet};
use std::fmt::Error;
use std::{collections::HashMap, sync::LazyLock};
use trace_middleware::trace_middleware;
use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    let config = load_config().unwrap();
    debug!("加载配置成功：{:#?}", config);
    config
});

fn env_filter(filter: EnvFilter) -> EnvFilter {
    filter
        .add_directive("opentelemetry=debug".parse().unwrap())
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("opentelemetry=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap())
        .add_directive("tower=off".parse().unwrap())
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (logger_provider, _guard) = init_log().unwrap();
    let pools: HashMap<String, Pool> = APP_CONFIG
        .modbus
        .configs
        .iter()
        .map(|config| {
            let mgr = ModbusManager {
                addr: config.address.to_string(),
                slave: config.slave_id,
            };
            let modbus_pool = Pool::builder(mgr).max_size(1).build().unwrap();
            (config.name.clone(), modbus_pool)
        })
        .collect();
    let server_url = &*APP_CONFIG.server.address;
    info!(name: "my-event", target: "my-target", "hello from {}. My price is {}", "apple", 1.99);
    let tracer_provider = init_traces().unwrap();
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(from_fn(trace_middleware))
            .app_data(web::Data::new(pools.clone()))
            .service(greet)
            .service(get_modbus_value)
    })
    .bind(server_url)?
    .run()
    .await?;
    // global::set_tracer_provider(tracer_provider);
    logger_provider.shutdown().unwrap();

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}

fn init_log() -> Result<(opentelemetry_sdk::logs::LoggerProvider, WorkerGuard), String> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    global::set_tracer_provider(init_traces().unwrap());

    let logger_provider: opentelemetry_sdk::logs::LoggerProvider = init_logs().unwrap();
    let otel_layer = OpenTelemetryTracingBridge::new(&logger_provider)
        .with_filter(env_filter(EnvFilter::new("debug")));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_filter(env_filter(EnvFilter::new("debug")));

    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // 创建一个文件输出层
    let file_layer = fmt::layer()
        .with_thread_names(true)
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_filter(env_filter(EnvFilter::new("debug")));

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .with(file_layer)
        .init();
    Ok((logger_provider, _guard))
}
