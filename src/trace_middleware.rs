use std::future::{ready, Ready};

use actix_web::{
    body::MessageBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    middleware::Next,
    Error,
};
use futures_util::future::LocalBoxFuture;
use opentelemetry::{
    global,
    trace::{Span, SpanKind, Status, Tracer},
};
use tracing::info;
pub struct Trace;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for Trace
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TraceMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TraceMiddleware { service }))
    }
}
pub struct TraceMiddleware<S> {
    service: S,
}
impl<S, B> Service<ServiceRequest> for TraceMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        println!("Hi from start. You requested: {}", req.path());

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            println!("Hi from response");
            Ok(res)
        })
    }
}
pub async fn trace_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let tracer = global::tracer("dice_server");
    let mut span = tracer
        .span_builder(format!("{} {}", req.method(), req.uri().path()))
        .with_kind(SpanKind::Server)
        .start(&tracer);
    let res = next.call(req).await;
    // post-processing
    match &res {
        Ok(ref response) => {
            if response.status().is_server_error() || response.status().is_client_error() {
                // 如果是 4xx 或 5xx 错误，标记 span 为 Error
                span.set_status(Status::error(response.status().to_string()));
            } else {
                // 请求成功，设置状态为 Ok
                span.set_status(Status::Ok);
            }
        }
        Err(err) => {
            // 如果请求处理出错，设置状态为 Error
            span.set_status(Status::error(err.to_string()));
        }
    }
    res
}
