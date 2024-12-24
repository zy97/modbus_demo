mod app_config;
mod modbus_manager;
mod server_router;
use actix_web::{middleware, web, App, HttpServer};
use app_config::{load_config, AppConfig};
use modbus_manager::{ModbusContext, Pool};
use server_router::{get_modbus_value, greet};
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};
use tracing::{debug, info};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, FmtSubscriber};
static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    // M3 Ultra takes about 16 million years in --release config
    let config = load_config().unwrap();
    debug!("加载配置成功：{:#?}", config);
    config
});
#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 创建一个文件输出层
    let file_layer = Layer::new().with_writer(non_blocking); // 输出到文件

    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish()
        .with(file_layer);
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

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

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pools.clone()))
            .service(greet)
            .service(get_modbus_value)
            .wrap(middleware::Logger::default())
    })
    .bind(server_url)?
    .run()
    .await
}
