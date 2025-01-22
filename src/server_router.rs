use std::{collections::HashMap, time::Duration};

use crate::{modbus_manager::ModbusManager, Pool};
use actix_web::{get, rt::time::timeout, web, Error, Responder};
// use backoff::ExponentialBackoff;
// use backoff::{retry, retry_notify};
// use backon::ExponentialBuilder;
// use backon::Retryable;
use deadpool::managed::{Manager, Object};
use serde::Serialize;
use tokio::time::error::Elapsed;
use tokio_modbus::client::Client;
use tokio_modbus::{client::Reader, ExceptionCode};
use tracing::{error, info};

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}
// #[tracing::instrument] //加上这个，就trace里就没有我自己记录的事件了，变成了其他库的事件
#[get("/modbus/{name}")]
pub async fn get_modbus_value(
    name: web::Path<String>,
    pools: web::Data<HashMap<String, Pool>>,
) -> Result<impl Responder, Error> {
    let name = name.as_str();
    let modbus = pools.get(name);
    match modbus {
        Some(modbus_context) => {
            let mut modbus: Object<ModbusManager> = modbus_context.get().await.unwrap();

            let values = timeout(
                Duration::from_secs(1),
                modbus.context.read_holding_registers(0, 20),
            )
            .await;
            // let values = read_data(modbus).await;
            return match values {
                Ok(Ok(Ok(values))) => Ok(web::Json(Response::success(values))),
                Ok(Ok(Err(err))) => {
                    error!("读取成功，但服务器返回错误：{:?}", err);
                    Ok(web::Json(Response::error(err.to_string())))
                }
                Ok(Err(err)) => {
                    //服务器主动关闭与客户端的连接会进入这个异常
                    error!("读取失败：{:?}", err);
                    modbus.status = false;
                    // let _ = Object::take(modbus);

                    Ok(web::Json(Response::error(err.to_string())))
                }
                Err(e) => {
                    error!("超时读取失败：{:?}", e);
                    modbus.status = false;
                    // let _ = Object::take(modbus);
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
