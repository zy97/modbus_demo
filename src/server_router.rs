use actix_web::{get, web, Error, Responder};
use serde::Serialize;
use tokio_modbus::client::Reader;
use tracing::error;

use crate::Pool;

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

#[get("/modbus/{name}")]
pub async fn get_modbus_value(
    name: web::Path<String>,
    tcp_pool: web::Data<Pool>,
) -> Result<impl Responder, Error> {
    let mut modbus = tcp_pool.get().await.unwrap();
    let values = modbus.read_holding_registers(0, 20).await;
    return match values {
        Ok(Ok(values)) => Ok(web::Json(Response::success(values))),
        Ok(Err(err)) => {
            error!("读取成功，但服务器返回错误：{:?}", err);
            Ok(web::Json(Response::error(err.to_string())))
        }
        Err(e) => {
            error!("读取失败：{:?}", e);
            Ok(web::Json(Response::error(e.to_string())))
        }
    };
}
#[derive(Serialize)]
struct Response<T> {
    success: bool,
    error: String,
    value: Option<T>,
}
impl<T> Response<T> {
    fn success(value: T) -> Self {
        Response {
            success: true,
            error: String::new(),
            value: Some(value),
        }
    }
    fn error(error: impl AsRef<str>) -> Self {
        Response {
            success: false,
            error: error.as_ref().into(),
            value: None,
        }
    }
}
