use actix_web::rt::time::timeout;
use client::Context;
use deadpool::managed::{self, RecycleError};
use std::{net::SocketAddr, time::Duration};
use tokio_modbus::prelude::*;
use tracing::{debug, error, info};
pub type Pool = managed::Pool<ModbusManager>;
#[derive(Clone, Debug)]
pub struct ModbusManager {
    pub addr: String,
    pub slave: u8,
}
pub struct Modbus {
    pub addr: String,
    pub slave: u8,
    pub context: Context,
    pub status: bool,
}
#[derive(Debug)]
pub enum Error {
    Fail,
}
impl managed::Manager for ModbusManager {
    type Type = Modbus;
    type Error = Error;

    async fn create(&self) -> Result<Modbus, Error> {
        let socket_addr = self.addr.parse::<SocketAddr>().unwrap();
        match timeout(
            Duration::from_millis(1000),
            tcp::connect_slave(socket_addr, Slave(self.slave)),
        )
        .await
        {
            Ok(Ok(context)) => {
                debug!("连接modbus:{},成功", self.addr);
                Ok(Modbus {
                    addr: self.addr.clone(),
                    slave: self.slave,
                    context: context,
                    status: true,
                })
            }
            _ => {
                error!("连接modbus:{}，失败", self.addr);
                Err(Error::Fail)
            }
        }
    }

    async fn recycle(
        &self,
        conn: &mut Modbus,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<Error> {
        //如果每次都需要连接一下在使用，则整体的效率会变慢一倍
        //所以应该在每次从池中取出来的modbus实例应该判断状态，在用重试机制调用
        match conn.status {
            true => Ok(()),
            _ => {
                conn.context.disconnect().await.unwrap();
                debug!("断开连接成功！");
                Err(RecycleError::Message(std::borrow::Cow::Borrowed(
                    "can't recycle",
                )))
            }
        }
    }
    fn detach(&self, _obj: &mut Self::Type) {}
}
impl ModbusManager {}

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
