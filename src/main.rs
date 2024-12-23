use actix_web::{get, middleware, web, App, HttpServer, Responder};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, FmtSubscriber};

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 创建一个文件输出层
    let file_layer = Layer::new().with_writer(non_blocking); // 输出到文件

    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish()
        .with(file_layer);
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    HttpServer::new(|| {
        App::new()
            .service(greet)
            .wrap(middleware::Logger::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
