use client::Context;
use deadpool::managed::{self};
use salvo::{affix, prelude::*, Request};
use serde::Serialize;
use std::collections::HashMap;
use std::{net::SocketAddr, time::Duration};
use tokio::time::timeout;
use tokio_modbus::prelude::*;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

static IP1: &'static str = "192.168.70.100";
static IP2: &'static str = "192.168.70.102";
#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref LOCATION: HashMap<&'static str, (&'static str, u16, &'static str)> = {
        let mut m = HashMap::new();
        m.insert("5504-1-1-1", (IP1, 5, "翻包区出口"));
        m.insert("5107-1-1-1", (IP1, 4, "一楼流水线入口"));
        m.insert("5106-1-1-1", (IP1, 3, "一楼流水线出口"));
        m.insert("5101-1-1-1", (IP1, 2, "二楼流水线靠近机房入口"));
        m.insert("5102-1-1-1", (IP1, 1, "二楼流水线远离机房入口"));
        m.insert("5103-1-1-1", (IP1, 0, "二楼流水线出库"));
        m.insert("5104-1-1-1", (IP2, 0, "成品流水线入口"));
        m.insert("5105-1-1-1", (IP2, 1, "成品流水线出口"));
        m
    };
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    let router = get_router();
    println!("{:?}", router);
    Server::new(acceptor).serve(router).await;
}
fn get_router() -> Router {
    let mut hash = HashMap::new();
    hash.insert(
        IP1.to_string(),
        Pool::builder(ModbusContext {
            ip: IP1.to_string(),
            port: 2000,
        })
        .build()
        .unwrap(),
    );
    hash.insert(
        IP2.to_string(),
        Pool::builder(ModbusContext {
            ip: IP2.to_string(),
            port: 2000,
        })
        .build()
        .unwrap(),
    );
    let contexts = Contexts { contexts: hash };
    let router = Router::with_path("location")
        .hoop(affix::inject(contexts))
        .push(Router::with_path("getall").get(locations))
        .push(Router::with_path("<location>").get(location));
    router
}
#[handler]
async fn location(req: &mut Request, depot: &mut Depot) -> Result<Json<LocationAvailable>, String> {
    let contexts = depot.obtain::<Contexts>().unwrap();
    let loc = req.param::<String>("location");
    if loc.is_none() {
        info!("未提供location值");
        return Ok(Json(LocationAvailable {
            location: String::new(),
            alias: String::new(),
            is_available: None,
        }));
    }
    let loc = loc.unwrap();
    let info = LOCATION.get(loc.as_str());
    if info.is_none() {
        info!("提供的location值无效");
        return Ok(Json(LocationAvailable {
            location: loc,
            alias: String::new(),
            is_available: None,
        }));
    }
    let (ip, reg, alias) = info.unwrap();
    let context = contexts.contexts.get(&ip.to_string()).unwrap();
    let mut context = context.get().await.ok();
    return Ok(Json(
        get_value(
            context.as_deref_mut(),
            alias.to_string(),
            loc,
            reg.to_owned(),
        )
        .await,
    ));
}

#[handler]
async fn locations(depot: &mut Depot) -> Result<Json<Vec<LocationAvailable>>, String> {
    let contexts = depot.obtain::<Contexts>().unwrap();
    let mut results = vec![];
    let mut hash = HashMap::new();

    for (loc, (ip, reg, alias)) in LOCATION.iter() {
        if !hash.contains_key(ip) {
            let ab = contexts.contexts.get(&ip.to_string()).unwrap();
            let context = ab.get().await.ok();
            hash.insert(ip, context);
        }
        let context = hash.get_mut(ip).unwrap();
        results.push(
            get_value(
                context.as_deref_mut(),
                alias.to_string(),
                loc.to_string(),
                reg.to_owned(),
            )
            .await,
        );
    }

    Ok(Json(results))
}
async fn get_value(
    context: Option<&mut Context>,
    alias: String,
    loc: String,
    reg: u16,
) -> LocationAvailable {
    match context {
        Some(context) => {
            let c = context.read_holding_registers(reg.to_owned(), 1).await;
            if let Ok(d) = c {
                LocationAvailable {
                    alias: alias.to_string(),
                    location: loc.to_string(),
                    is_available: Some(d[0] == 6),
                }
            } else {
                info!("不能读取数据从地址{}", reg);
                LocationAvailable {
                    alias: alias.to_string(),
                    location: loc.to_string(),
                    is_available: Some(false),
                }
            }
        }
        None => LocationAvailable {
            alias: alias.to_string(),
            location: loc.to_string(),
            is_available: None,
        },
    }
}

#[derive(Serialize)]
struct LocationAvailable {
    location: String,
    is_available: Option<bool>,
    alias: String,
}
#[derive(Debug)]
enum Error {
    Fail,
}
#[derive(Clone)]
struct ModbusContext {
    ip: String,
    port: u32,
}
impl managed::Manager for ModbusContext {
    type Type = Context;
    type Error = Error;

    async fn create(&self) -> Result<Context, Error> {
        let addr = format!("{}:{}", self.ip, self.port);
        let socket_addr = addr.parse::<SocketAddr>().unwrap();
        match timeout(Duration::from_millis(100), tcp::connect(socket_addr)).await {
            Ok(Ok(context)) => {
                info!("连接到{}:{}", self.ip, self.port);
                Ok(context)
            }
            _ => {
                info!("不能连接到{}:{}", self.ip, self.port);
                Err(Error::Fail)
            }
        }
    }

    async fn recycle(
        &self,
        _: &mut Context,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<Error> {
        Ok(())
    }
}

type Pool = managed::Pool<ModbusContext>;

#[allow(dead_code)]
#[derive(Clone)]
struct Contexts {
    contexts: HashMap<String, Pool>,
}
