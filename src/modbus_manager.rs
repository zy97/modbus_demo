use actix_web::rt::time::timeout;
use client::Context;
use deadpool::managed::{self, RecycleError};
use std::{net::SocketAddr, time::Duration};
use tokio_modbus::prelude::*;
use tracing::{debug, error, info};
pub type Pool = managed::Pool<ModbusContext>;
#[derive(Clone, Debug)]
pub struct ModbusContext {
    pub addr: String,
    pub slave: u8,
}
#[derive(Debug)]
pub enum Error {
    Fail,
}
impl managed::Manager for ModbusContext {
    type Type = Context;
    type Error = Error;

    async fn create(&self) -> Result<Context, Error> {
        let socket_addr = self.addr.parse::<SocketAddr>().unwrap();
        match timeout(
            Duration::from_millis(1000),
            tcp::connect_slave(socket_addr, Slave(self.slave)),
        )
        .await
        {
            Ok(Ok(context)) => {
                debug!("连接modbus:{},成功", self.addr);
                Ok(context)
            }
            _ => {
                error!("连接modbus:{}，失败", self.addr);
                Err(Error::Fail)
            }
        }
    }

    async fn recycle(
        &self,
        conn: &mut Context,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<Error> {
        match is_connection_alive(conn).await {
            true => Ok(()),
            _ => {
                conn.disconnect().await.unwrap();
                Err(RecycleError::Message(std::borrow::Cow::Borrowed(
                    "can't recycle",
                )))
            }
        }
    }
    fn detach(&self, _obj: &mut Self::Type) {
        info!("断开连接成功！");
    }
}
impl ModbusContext {}

async fn is_connection_alive(context: &mut Context) -> bool {
    match timeout(
        Duration::from_millis(100),
        context.read_holding_registers(0x00, 1),
    )
    .await
    {
        Ok(Ok(_)) => true,
        _ => {
            error!("connect error!");
            false
        }
    }
}
