use std::{collections::HashMap, time::Duration};

use actix_web::{get, rt::time::timeout, web, Error, Responder};
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
    pools: web::Data<HashMap<String, Pool>>,
) -> Result<impl Responder, Error> {
    let name = name.as_str();
    let modbus = pools.get(name);
    match modbus {
        Some(modbus) => {
            let mut modbus = modbus.get().await.unwrap();
            let values =
                timeout(Duration::from_secs(1), modbus.read_holding_registers(0, 20)).await;
            return match values {
                Ok(Ok(Ok(values))) => Ok(web::Json(Response::success(values))),
                Ok(Ok(Err(err))) => {
                    error!("读取成功，但服务器返回错误：{:?}", err);
                    Ok(web::Json(Response::error(err.to_string())))
                }
                Ok(Err(err)) => {
                    error!("读取失败：{:?}", err);
                    Ok(web::Json(Response::error(err.to_string())))
                }
                Err(e) => {
                    error!("超时读取失败：{:?}", e);
                    Ok(web::Json(Response::error(e.to_string())))
                }
            };
        }
        None => Ok(web::Json(Response::error(format!(
            "不存在配置名为{}的modbus配置！",
            name,
        )))),
    }
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
