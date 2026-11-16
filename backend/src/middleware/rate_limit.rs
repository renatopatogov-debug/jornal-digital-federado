use crate::middleware::rbac::extrair_ip_cliente;
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
struct WindowBucket {
    count: u32,
    window_start: Instant,
}

#[derive(Clone)]
pub struct RateLimiter {
    global_limit: u32,
    auth_limit: u32,
    window: Duration,
    buckets: Arc<Mutex<HashMap<(IpAddr, bool), WindowBucket>>>,
}

impl RateLimiter {
    pub fn new(global_limit: u32, auth_limit: u32) -> Self {
        Self {
            global_limit: global_limit.max(1),
            auth_limit: auth_limit.max(1),
            window: Duration::from_secs(60),
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn check(&self, ip: IpAddr, is_auth_route: bool) -> bool {
        let now = Instant::now();
        let limit = if is_auth_route {
            self.auth_limit
        } else {
            self.global_limit
        };

        let mut map = self.buckets.lock().await;
        if map.len() > 4096 {
            let window = self.window;
            map.retain(|_, v| now.duration_since(v.window_start) < window);
        }

        let entry = map.entry((ip, is_auth_route)).or_insert(WindowBucket {
            count: 0,
            window_start: now,
        });

        if now.duration_since(entry.window_start) >= self.window {
            entry.count = 1;
            entry.window_start = now;
            return true;
        }

        if entry.count >= limit {
            return false;
        }

        entry.count += 1;
        true
    }
}

pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    let is_auth_route = path.starts_with("/api/v1/auth");
    let ip = extrair_ip_cliente(req.headers());

    if !limiter.check(ip, is_auth_route).await {
        let mut resp = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "erro": true,
                "mensagem": "Limite de requisições por IP excedido. Aguarde antes de tentar novamente.",
                "status_code": 429
            })),
        )
            .into_response();
        resp.headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        return resp;
    }

    next.run(req).await
}
