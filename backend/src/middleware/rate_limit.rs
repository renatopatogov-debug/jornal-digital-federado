use crate::middleware::rbac::extrair_ip_requisicao;
use crate::services::redis_cache::RedisService;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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
    trust_proxy: bool,
    window: Duration,
    redis: RedisService,
    buckets: Arc<Mutex<HashMap<(IpAddr, bool), WindowBucket>>>,
}

impl RateLimiter {
    pub fn new(global_limit: u32, auth_limit: u32, trust_proxy: bool) -> Self {
        Self::new_with_redis(global_limit, auth_limit, trust_proxy, None)
    }

    pub fn new_with_redis(
        global_limit: u32,
        auth_limit: u32,
        trust_proxy: bool,
        redis_url: Option<&str>,
    ) -> Self {
        let redis = match redis_url.map(str::trim).filter(|s| !s.is_empty()) {
            Some(url) => RedisService::from_url(url),
            None => RedisService::disabled(),
        };
        Self {
            global_limit: global_limit.max(1),
            auth_limit: auth_limit.max(1),
            trust_proxy,
            window: Duration::from_secs(60),
            redis,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn check(&self, ip: IpAddr, is_auth_route: bool) -> bool {
        let limit = if is_auth_route {
            self.auth_limit
        } else {
            self.global_limit
        };

        // Tenta rate-limiting distribuído no Redis 7 (multi-réplica) com fallback transparente em memória
        if self.redis.is_configured() {
            let minute_slot = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() / 60)
                .unwrap_or(0);
            let escopo = if is_auth_route { "auth" } else { "global" };
            let key = format!("jornal:rl:{}:{}:{}", escopo, ip, minute_slot);
            if let Some(current_count) = self.redis.incr_window(&key, 65).await {
                return current_count <= u64::from(limit);
            }
        }

        let now = Instant::now();

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
    let peer_ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip());
    let ip = extrair_ip_requisicao(req.headers(), peer_ip, limiter.trust_proxy);

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
