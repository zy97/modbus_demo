use client::Context;
use deadpool::managed::{self, Manager, Object};
use salvo::{affix, prelude::*, server::ServerHandle, Request};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use tokio::{signal, time::timeout};
use tokio_modbus::prelude::*;
use tracing::info;
use tracing_subscriber::FmtSubscriber;
static IP1: &'static str = "192.168.70.100";
static IP2: &'static str = "192.168.70.102";
#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref CAN_TAKE_LOCATION: HashMap<&'static str, (&'static str, u16, &'static str)> = {
        let mut m = HashMap::new();
        // m.insert("5504-1-1-1", (IP1, 5, "翻包区出口"));
        // m.insert("5106-1-1-1", (IP1, 3, "一楼流水线出口"));
        m.insert("5101-1-1-1", (IP1, 2, "二楼流水线靠近机房入口"));
        // m.insert("5102-1-1-1", (IP1, 1, "二楼流水线远离机房入口"));
        // m.insert("5105-1-1-1", (IP2, 1, "成品流水线出口"));
        m
    };
    static ref CAN_PUT_DOWN_LOCATIONS: HashMap<&'static str, (&'static str, u16, &'static str, &'static str)> = {
        let mut m = HashMap::new();
        m.insert("SC01-2", (IP1, 4, "一楼流水线入口", "5107-1-1-1"));
        m.insert("SC02-3", (IP1, 0, "二楼流水线出库", "5103-1-1-1"));
        m.insert("SC02-4", (IP2, 0, "成品流水线入口", "5104-1-1-1"));
        m
    };
    static ref CONTROL_MAP: HashMap<(&'static str, &'static str), (&'static str, u16, u16, &'static str)> = {
        let mut m = HashMap::new();
        m.insert(
            ("5104-1-1-1", "5104-1-1-1"),
            (IP2, 10, 6, "成品流水线不通过"),
        );
        m.insert(
            ("5104-1-1-1", "5105-1-1-1"),
            (IP2, 10, 16, "成品流水线通过"),
        );

        m.insert(
            ("5501-1-1-1", "5106-1-1-1"),
            (IP1, 0, 6, "2楼半成品流水线副出口送至一楼入库接驳点"),
        );
        m.insert(
            ("5501-1-1-1", "5504-1-1-1"),
            (IP1, 0, 6, "2楼半成品流水线副出口送至一楼翻包区"),
        );

        m.insert(
            ("5103-1-1-1", "5106-1-1-1"),
            (IP1, 0, 16, "2楼半成品流水线主出口送至一楼入库接驳点"),
        );
        m.insert(
            ("5103-1-1-1", "5504-1-1-1"),
            (IP1, 0, 16, "2楼半成品流水线主出口送至一楼翻包区"),
        );
        m.insert(
            ("5103-1-1-1", "5505-1-1-1"),
            (IP1, 0, 6, "2楼半成品流水线主出口送至拆箱区"),
        );
        m.insert(
            ("5103-1-1-1", "5101-1-1-1"),
            (IP1, 0, 26, "2楼半成品流水线主出口送至靠近机房入库点"),
        );
        m.insert(
            ("5103-1-1-1", "5102-1-1-1"),
            (IP1, 0, 26, "2楼半成品流水线主出口送至离近机房入库点"),
        );

        m.insert(
            ("5502-1-1-1", "5505-1-1-1"),
            (IP1, 0, 6, "2楼半成品流水线交叉口送至拆箱区"),
        );
        m.insert(
            ("5502-1-1-1", "5101-1-1-1"),
            (IP1, 0, 26, "2楼半成品流水线交叉口送至靠近机房入库点"),
        );
        m.insert(
            ("5502-1-1-1", "5102-1-1-1"),
            (IP1, 0, 16, "2楼半成品流水线交叉口送远离近机房入库点"),
        );

        m.insert(
            ("5101-1-1-1", "5101-1-1-1"),
            (IP1, 0, 6, "2楼半成品流水线靠近机房入库点到终点"),
        );
        m.insert(
            ("5102-1-1-1", "5102-1-1-1"),
            (IP1, 0, 6, "2楼半成品流水线远离机房入库点到终点"),
        );

        m.insert(
            ("5106-1-1-1", "5504-1-1-1"),
            (IP1, 0, 6, "1楼流水线出口到翻包区"),
        );
        m.insert(
            ("5106-1-1-1", "5106-1-1-1"),
            (IP1, 0, 16, "1楼流水线出口到出口点"),
        );

        m.insert(
            ("5107-1-1-1", "5504-1-1-1"),
            (IP1, 0, 6, "1楼流水线入口到翻包区"),
        );
        m.insert(
            ("5107-1-1-1", "5505-1-1-1"),
            (IP1, 0, 16, "1楼流水线入口到拆箱区"),
        );
        m.insert(
            ("5107-1-1-1", "5101-1-1-1"),
            (IP1, 0, 16, "1楼流水线入口到2楼出库接驳点"),
        );
        m.insert(
            ("5107-1-1-1", "5102-1-1-1"),
            (IP1, 0, 16, "1楼流水线入口到2楼出库接驳点"),
        );

        m.insert(
            ("5503-1-1-1", "5106-1-1-1"),
            (IP1, 0, 6, "1楼翻包区到一楼入库接驳点"),
        );
        m.insert(
            ("5503-1-1-1", "5505-1-1-1"),
            (IP1, 0, 16, "1楼流水线入口到2楼拆箱区"),
        );
        m.insert(
            ("5503-1-1-1", "5101-1-1-1"),
            (IP1, 0, 16, "1楼流水线入口到2楼入库接驳点"),
        );
        m.insert(
            ("5503-1-1-1", "5102-1-1-1"),
            (IP1, 0, 16, "1楼流水线入口到2楼入库接驳点"),
        );
        m
    };
    static ref POOLS: Contexts = {
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
        contexts
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
    let server = Server::new(acceptor);
    let handle = server.handle();
    tokio::spawn(listen_shutdown_signal(handle));
    server.serve(router).await;
}
fn get_router() -> Router {
    let mut hash = HashMap::new();
    hash.insert(
        IP1.to_string(),
        Pool::builder(ModbusContext {
            ip: IP1.to_string(),
            port: 2000,
        })
        .max_size(1)
        .build()
        .unwrap(),
    );
    hash.insert(
        IP2.to_string(),
        Pool::builder(ModbusContext {
            ip: IP2.to_string(),
            port: 2000,
        })
        .max_size(1)
        .build()
        .unwrap(),
    );
    let contexts = Contexts { contexts: hash };
    let router = Router::with_path("location")
        .hoop(affix::inject(contexts))
        .push(Router::with_path("getall").get(get_all_locations))
        .push(Router::with_path("").post(locations));
    router
}
async fn listen_shutdown_signal(handle: ServerHandle) {
    // Wait Shutdown Signal
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(windows)]
    let terminate = async {
        signal::windows::ctrl_c()
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => println!("ctrl_c signal received"),
        _ = terminate => println!("terminate signal received"),
    };

    // Graceful Shutdown Server
    handle.stop_graceful(None);
}

#[handler]
async fn locations(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<Vec<LocationAvailable>>, String> {
    let mut results = vec![];
    let contexts = depot.obtain::<Contexts>().unwrap();
    let request_locations = req.parse_json::<Locations>().await;
    if request_locations.is_err() {
        info!("未提供location值");
        return Ok(Json(results));
    }
    let request_locations = request_locations.unwrap();
    for location in request_locations.locations {
        let info = CAN_TAKE_LOCATION.get(location.as_str());
        if info.is_none() {
            info!("提供的location值无效");
            results.push(LocationAvailable {
                location: location.clone(),
                alias: String::new(),
                is_available: None,
            });
        }
        let (ip, reg, alias) = info.unwrap();
        let pool = contexts.contexts.get(&ip.to_string()).unwrap();
        info!("起始状态：{:#?}", pool.status());
        let mut context = pool.get().await.ok();
        get_value1(
            context,
            alias.to_string(),
            location,
            reg.to_owned(),
            ip.to_string(),
        );
        info!("获取池之后状态：{:#?}", pool.status());
        results.push(
            get_value(
                context.as_deref_mut(),
                alias.to_string(),
                location,
                reg.to_owned(),
                ip.to_string(),
            )
            .await
            .0,
        );
        let a = Object::take(context.unwrap());
        // drop(context);
        // pool.close();
        // pool.resize(1);

        info!("释放池之后状态：{:#?}", pool.status());
    }

    Ok(Json(results))
}

#[handler]
async fn get_all_locations(depot: &mut Depot) -> Result<Json<Vec<LocationAvailable>>, String> {
    let mut results = vec![];
    let mut hash = HashMap::new();
    let mut reconnect = vec![];
    for (loc, (ip, reg, alias)) in CAN_TAKE_LOCATION.iter() {
        if !hash.contains_key(ip) {
            let ab = POOLS.contexts.get(&ip.to_string()).unwrap();
            let context = ab.get().await.ok();
            hash.insert(ip, context);
        }
        let context = hash.get_mut(ip).unwrap();
        let data = get_value(
            context.as_deref_mut(),
            alias.to_string(),
            loc.to_string(),
            reg.to_owned(),
            ip.to_string(),
        )
        .await;
        if data.1 .1 == true {
            reconnect.push(data.1)
        }
        results.push(data.0);
    }
    reconnect.dedup();
    for (ip, pool) in POOLS.contexts.clone() {
        let a = pool.get().await.unwrap();
        info!("关闭前的状态：{:#?}", pool.status());
        drop(a);
        info!("关闭后的状态：{:#?}", pool.status());
    }
    // for con in reconnect {
    //     let c = POOLS.contexts.get(&con.0).unwrap();
    //     info!("状态：{:#?}", c.status());
    //     drop(c);
    //     info!("关闭后的状态：{:#?}", c.status());
    // }
    Ok(Json(results))
}
async fn get_value(
    context: Option<&mut Context>,
    alias: String,
    loc: String,
    reg: u16,
    ip: String,
) -> (LocationAvailable, (String, bool)) {
    match context {
        Some(context) => {
            let c = context.read_holding_registers(reg.to_owned(), 1).await;
            if let Ok(d) = c {
                (
                    LocationAvailable {
                        alias: alias.to_string(),
                        location: loc.to_string(),
                        is_available: Some(d[0] == 6),
                    },
                    (ip, false),
                )
            } else {
                info!("不能读取数据从地址{}", reg);
                (
                    LocationAvailable {
                        alias: alias.to_string(),
                        location: loc.to_string(),
                        is_available: Some(false),
                    },
                    (ip, true),
                )
            }
        }
        None => (
            LocationAvailable {
                alias: alias.to_string(),
                location: loc.to_string(),
                is_available: None,
            },
            (ip, true),
        ),
    }
}

async fn get_value1(
    context: Option<Object<ModbusContext>>,
    alias: String,
    loc: String,
    reg: u16,
    ip: String,
) -> (LocationAvailable, (String, bool)) {
    match context {
        Some(mut context) => {
            let c = context.read_holding_registers(reg.to_owned(), 1).await;
            if let Ok(d) = c {
                (
                    LocationAvailable {
                        alias: alias.to_string(),
                        location: loc.to_string(),
                        is_available: Some(d[0] == 6),
                    },
                    (ip, false),
                )
            } else {
                info!("不能读取数据从地址{}", reg);
                (
                    LocationAvailable {
                        alias: alias.to_string(),
                        location: loc.to_string(),
                        is_available: Some(false),
                    },
                    (ip, true),
                )
            }
        }
        None => (
            LocationAvailable {
                alias: alias.to_string(),
                location: loc.to_string(),
                is_available: None,
            },
            (ip, true),
        ),
    }
}

#[derive(Serialize)]
struct LocationAvailable {
    location: String,
    is_available: Option<bool>,
    alias: String,
}
#[derive(Deserialize)]
struct Locations {
    locations: Vec<String>,
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
    fn detach(&self, _obj: &mut Self::Type) {}
}

type Pool = managed::Pool<ModbusContext>;

#[allow(dead_code)]
#[derive(Clone)]
struct Contexts {
    contexts: HashMap<String, Pool>,
}
