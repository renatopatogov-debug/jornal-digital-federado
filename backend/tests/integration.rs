use axum::http::StatusCode;
use chrono::Utc;
use jornal_backend::{
    config::AppConfig,
    routes::build_app_router,
    services::{crypto::CryptoService, session::SessionManager},
};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

struct HttpResponseData {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl HttpResponseData {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|e| panic!("JSON inválido ({}): {}", e, self.body))
    }
}

fn config_teste(global_rpm: u32, auth_rpm: u32, trust_proxy: bool) -> AppConfig {
    AppConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        database_url: "postgres://postgres:postgres@localhost:5432/jornal_test".to_string(),
        redis_url: String::new(),
        database_max_connections: 5,
        modo_eleitoral_default: false,
        max_ativacao_semanal_lider_municipal: 50,
        rate_limit_votos_por_minuto: 10,
        rate_limit_global_per_minute: global_rpm,
        rate_limit_auth_per_minute: auth_rpm,
        feed_cache_ttl_seconds: 20,
        cors_allowed_origins: vec![
            "http://localhost:3000".to_string(),
            "http://127.0.0.1:3000".to_string(),
        ],
        cookie_secure: false,
        cookie_same_site: "Strict".to_string(),
        trust_proxy,
        aes_previous_master_key: None,
    }
}

async fn spawn_server(pool: PgPool, config: &AppConfig) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind efêmero local");
    let addr = listener.local_addr().expect("local_addr válido");
    let app = build_app_router(pool, config);
    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });
    addr
}

fn decodificar_chunked(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some((len_line, after_len)) = rest.split_once("\r\n") {
        let len_hex = len_line.split(';').next().unwrap_or("").trim();
        let Ok(size) = usize::from_str_radix(len_hex, 16) else {
            break;
        };
        if size == 0 {
            break;
        }
        if after_len.len() < size {
            out.push_str(after_len);
            break;
        }
        out.push_str(&after_len[..size]);
        rest = after_len[size..].strip_prefix("\r\n").unwrap_or("");
    }
    out
}

async fn send_http(
    addr: SocketAddr,
    method: &str,
    path: &str,
    extra_headers: &[(&str, &str)],
    body: Option<&str>,
) -> HttpResponseData {
    let mut stream = TcpStream::connect(addr)
        .await
        .expect("conectar servidor de teste");
    let body_bytes = body.unwrap_or("").as_bytes();

    let mut req = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        method, path, addr
    );
    for (k, v) in extra_headers {
        req.push_str(&format!("{}: {}\r\n", k, v));
    }
    if body.is_some() {
        req.push_str("Content-Type: application/json\r\n");
        req.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
    }
    req.push_str("\r\n");

    stream
        .write_all(req.as_bytes())
        .await
        .expect("escrever headers HTTP");
    if !body_bytes.is_empty() {
        stream
            .write_all(body_bytes)
            .await
            .expect("escrever corpo HTTP");
    }
    stream.flush().await.expect("flush TCP");

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("ler resposta HTTP");
    let text = String::from_utf8_lossy(&buf);

    let (head, raw_body) = text
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("Resposta HTTP malformada: {}", text));

    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let headers: Vec<(String, String)> = lines
        .filter_map(|line| {
            let (k, v) = line.split_once(':')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect();

    let is_chunked = headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("transfer-encoding") && v.to_lowercase().contains("chunked"));

    let decoded_body = if is_chunked {
        decodificar_chunked(raw_body)
    } else {
        raw_body.to_string()
    };

    HttpResponseData {
        status,
        headers,
        body: decoded_body,
    }
}

fn extrair_cookie_sessao(resp: &HttpResponseData) -> Option<String> {
    resp.headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .find_map(|(_, v)| v.split(';').next().map(|s| s.trim().to_string()))
        .filter(|c| c.starts_with("jornal_session=") || c.starts_with("__Host-jornal_session="))
}

fn decodificar_base32_teste(s: &str) -> Vec<u8> {
    let mut buffer: u32 = 0;
    let mut bits_left: u8 = 0;
    let mut out = Vec::new();
    for ch in s.bytes() {
        let val = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'2'..=b'7' => ch - b'2' + 26,
            _ => continue,
        };
        buffer = (buffer << 5) | u32::from(val);
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            out.push(((buffer >> bits_left) & 0xff) as u8);
        }
    }
    out
}

#[tokio::test]
async fn openapi_e_docs_respondem_com_especificacao_e_headers_seguranca() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
        .expect("pool lazy válido");
    let addr = spawn_server(pool, &config_teste(120, 15, false)).await;

    let spec_resp = send_http(addr, "GET", "/api/openapi.json", &[], None).await;
    assert_eq!(spec_resp.status, StatusCode::OK.as_u16());
    assert_eq!(spec_resp.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(spec_resp.header("x-frame-options"), Some("DENY"));

    let spec = spec_resp.json();
    assert_eq!(spec["openapi"].as_str(), Some("3.0.3"));
    assert!(spec["paths"]["/api/v1/auth/login"].is_object());
    assert!(spec["paths"]["/api/v1/auth/2fa/setup"].is_object());
    assert!(spec["paths"]["/api/v1/auth/2fa/confirmar"].is_object());
    assert!(spec["paths"]["/api/v1/auth/sessoes"].is_object());
    assert!(spec["paths"]["/api/v1/noticias/acervo"].is_object());
    assert!(spec["paths"]["/api/v1/escalonamentos"].is_object());
    assert!(spec["paths"]["/api/v1/escalonamentos/{id}/avaliar"].is_object());
    assert!(spec["paths"]["/api/v1/noticias/{id}/voto"].is_object());
    assert!(spec["paths"]["/api/v1/infra/rotacionar-chaves-totp"].is_object());
    assert!(spec["paths"]["/api/v1/infra/modo-eleitoral"].is_object());
    assert!(spec["paths"]["/api/v1/infra/auditoria"].is_object());

    let docs_resp = send_http(addr, "GET", "/api/docs", &[], None).await;
    assert_eq!(docs_resp.status, StatusCode::OK.as_u16());
    assert!(docs_resp.body.contains("/api/openapi.json"));
}

#[tokio::test]
async fn rotas_protegidas_exigem_cookie_de_sessao() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
        .expect("pool lazy válido");
    let addr = spawn_server(pool, &config_teste(120, 15, false)).await;

    let rotas = [
        ("GET", "/api/v1/auth/me", None),
        ("GET", "/api/v1/auth/sessoes", None),
        ("POST", "/api/v1/auth/2fa/setup", None),
        (
            "POST",
            "/api/v1/auth/2fa/confirmar",
            Some(r#"{"codigo_totp":"123456"}"#),
        ),
        ("POST", "/api/v1/auth/sessoes/revogar-todas", Some("{}")),
        ("GET", "/api/v1/noticias/acervo", None),
        (
            "POST",
            "/api/v1/noticias/preview-sanitizado",
            Some(r#"{"conteudo_html":"<p>oi</p>"}"#),
        ),
        ("GET", "/api/v1/escalonamentos", None),
        (
            "POST",
            "/api/v1/escalonamentos/00000000-0000-0000-0000-000000000001/avaliar",
            Some(r#"{"aprovado":true}"#),
        ),
        (
            "POST",
            "/api/v1/infra/rotacionar-chaves-totp",
            Some("{}"),
        ),
        ("GET", "/api/v1/infra/modo-eleitoral", None),
        (
            "POST",
            "/api/v1/infra/modo-eleitoral",
            Some(r#"{"ativo":true,"motivo":"Teste"}"#),
        ),
        ("GET", "/api/v1/infra/auditoria", None),
        ("POST", "/api/v1/usuarios/ativar", Some("{}")),
    ];

    for (method, path, body) in rotas {
        let resp = send_http(addr, method, path, &[], body).await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED.as_u16(),
            "Rota {} {} deveria retornar 401 sem cookie",
            method,
            path
        );
        assert_eq!(
            resp.header("cache-control"),
            Some("no-store, no-cache, must-revalidate, private")
        );
    }
}

#[tokio::test]
async fn csrf_origin_nao_autorizada_bloqueada_em_post_com_cookie() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
        .expect("pool lazy válido");
    let addr = spawn_server(pool, &config_teste(120, 15, false)).await;

    let resp = send_http(
        addr,
        "POST",
        "/api/v1/noticias",
        &[
            ("Cookie", "__Host-jornal_session=token_ficticio"),
            ("Origin", "https://atacante.invalid"),
        ],
        Some("{}"),
    )
    .await;

    assert_eq!(resp.status, StatusCode::FORBIDDEN.as_u16());
    assert!(resp.body.contains("Bloqueio CSRF"));
}

#[tokio::test]
async fn rate_limiter_por_ip_bloqueia_rajada_com_429() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
        .expect("pool lazy válido");
    let addr = spawn_server(pool, &config_teste(10, 3, true)).await;

    for tentativa in 1..=3 {
        let resp = send_http(
            addr,
            "POST",
            "/api/v1/auth/login",
            &[("X-Forwarded-For", "203.0.113.10")],
            Some(r#"{"email":"","senha":""}"#),
        )
        .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST.as_u16(),
            "Tentativa {} dentro da cota não deve sofrer rate limit",
            tentativa
        );
    }

    let bloqueada = send_http(
        addr,
        "POST",
        "/api/v1/auth/login",
        &[("X-Forwarded-For", "203.0.113.10")],
        Some(r#"{"email":"","senha":""}"#),
    )
    .await;
    assert_eq!(bloqueada.status, StatusCode::TOO_MANY_REQUESTS.as_u16());
    assert_eq!(bloqueada.header("retry-after"), Some("60"));

    // Outro IP permanece liberado quando TRUST_PROXY=true
    let outro_ip = send_http(
        addr,
        "POST",
        "/api/v1/auth/login",
        &[("X-Forwarded-For", "203.0.113.11")],
        Some(r#"{"email":"","senha":""}"#),
    )
    .await;
    assert_eq!(outro_ip.status, StatusCode::BAD_REQUEST.as_u16());
}

#[tokio::test]
async fn rate_limiter_ignora_xff_forjado_quando_trust_proxy_desabilitado() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
        .expect("pool lazy válido");
    let addr = spawn_server(pool, &config_teste(10, 3, false)).await;

    for i in 1..=3 {
        let xff_forjado = format!("198.51.100.{}", i);
        let resp = send_http(
            addr,
            "POST",
            "/api/v1/auth/login",
            &[("X-Forwarded-For", &xff_forjado)],
            Some(r#"{"email":"","senha":""}"#),
        )
        .await;
        assert_eq!(resp.status, StatusCode::BAD_REQUEST.as_u16());
    }

    // 4a tentativa com X-Forwarded-For diferente ainda conta no peer_ip (127.0.0.1) e recebe 429
    let bloqueada = send_http(
        addr,
        "POST",
        "/api/v1/auth/login",
        &[("X-Forwarded-For", "198.51.100.99")],
        Some(r#"{"email":"","senha":""}"#),
    )
    .await;
    assert_eq!(bloqueada.status, StatusCode::TOO_MANY_REQUESTS.as_u16());
}

#[tokio::test]
async fn verify_2fa_sem_desafio_valido_retorna_401() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
        .expect("pool lazy válido");
    let addr = spawn_server(pool, &config_teste(120, 15, false)).await;

    let resp = send_http(
        addr,
        "POST",
        "/api/v1/auth/2fa/verify",
        &[],
        Some(&json!({
            "usuario_id": Uuid::new_v4(),
            "codigo_totp": "123456",
            "desafio_2fa_token": "forjado:9999999999:deadbeef"
        }).to_string()),
    )
    .await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED.as_u16());
}

#[tokio::test]
#[ignore = "requer PostgreSQL 16 ativo com migrations aplicadas (executado em database-integrity)"]
async fn fluxo_completo_login_2fa_noticia_voto_hype_e_rotacao() {
    if std::env::var("RUN_E2E_DB_TESTS").as_deref() != Ok("1") {
        return;
    }

    std::env::set_var("COOKIE_SECURE", "false");
    std::env::set_var("COOKIE_SAME_SITE", "Strict");
    std::env::set_var("AES_256_GCM_MASTER_KEY", "chave-ativa-e2e-2026");
    std::env::set_var("AES_256_GCM_PREVIOUS_MASTER_KEY", "chave-antiga-e2e-2025");

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres_ci@localhost:5432/jornal_federado_ci".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("conectar PostgreSQL para teste E2E");

    let mut e2e_cfg = config_teste(500, 200, false);
    e2e_cfg.redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let addr = spawn_server(pool.clone(), &e2e_cfg).await;

    // 0. Healthcheck e Métricas validam conexão ativa com PostgreSQL 16 + Redis 7
    let health_resp = send_http(addr, "GET", "/health", &[], None).await;
    assert_eq!(health_resp.status, StatusCode::OK.as_u16());
    assert_eq!(health_resp.json()["redis_conectado"].as_bool(), Some(true));

    let metrics_resp = send_http(addr, "GET", "/metrics", &[], None).await;
    assert_eq!(metrics_resp.status, StatusCode::OK.as_u16());
    assert!(metrics_resp.body.contains("jornal_redis_up 1"));

    // 1. Login SuperAdmin seedado e rotação de envelope TOTP cifrado com chave antiga
    let redator_id = Uuid::new_v4();
    let redator_email = format!("redator.{}@jornalfederado.org.br", redator_id.simple());
    let redator_senha = "SenhaForte@E2E#2026";
    let senha_hash = CryptoService::gerar_hash_senha(redator_senha, "e2esalt01");
    let totp_secret = b"SEGREDO_TOTP_E2E_PARTIDO_35";
    let chave_antiga = CryptoService::from_raw_secret("chave-antiga-e2e-2025");
    let envelope_antigo = chave_antiga
        .cifrar_envelope_aleatorio(totp_secret)
        .expect("cifra antiga ok");

    sqlx::query(
        r#"
        INSERT INTO usuarios (
            id, nome, email, senha_hash, cargo_id, escopo, ibge_codigo,
            filiado_verificado, status, totp_secret_cifrado, totp_habilitado
        )
        VALUES ($1, 'Redator Teste E2E', $2, $3, 7, 'MUNICIPAL'::tipo_escopo, '3550308', TRUE, 'ATIVO'::status_usuario, $4, TRUE)
        "#,
    )
    .bind(redator_id)
    .bind(&redator_email)
    .bind(&senha_hash)
    .bind(&envelope_antigo)
    .execute(&pool)
    .await
    .expect("inserir redator de teste");

    let login_admin = send_http(
        addr,
        "POST",
        "/api/v1/auth/login",
        &[],
        Some(r#"{"email":"superadmin@jornalfederado.org.br","senha":"JornalFed@2026#Admin!"}"#),
    )
    .await;
    assert_eq!(login_admin.status, StatusCode::OK.as_u16());
    let admin_cookie = extrair_cookie_sessao(&login_admin).expect("cookie superadmin emitido");

    let rotacao_resp = send_http(
        addr,
        "POST",
        "/api/v1/infra/rotacionar-chaves-totp",
        &[("Cookie", &admin_cookie)],
        Some("{}"),
    )
    .await;
    assert_eq!(rotacao_resp.status, StatusCode::OK.as_u16());
    let rotacao_json = rotacao_resp.json();
    assert!(rotacao_json["registros_rotacionados"].as_u64().unwrap_or(0) >= 1);

    // 2. Login do redator com 2FA TOTP SHA-256 + verificação SameSite=Strict
    let login_redator = send_http(
        addr,
        "POST",
        "/api/v1/auth/login",
        &[],
        Some(&json!({ "email": redator_email, "senha": redator_senha }).to_string()),
    )
    .await;
    assert_eq!(login_redator.status, StatusCode::OK.as_u16());
    let login_redator_json = login_redator.json();
    assert_eq!(login_redator_json["requer_2fa"].as_bool(), Some(true));
    let desafio_2fa_token = login_redator_json["desafio_2fa_token"]
        .as_str()
        .expect("desafio_2fa_token emitido pelo login")
        .to_string();

    let agora_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("relogio valido")
        .as_secs();
    let codigo_totp = CryptoService::gerar_totp_sha256(totp_secret, agora_secs / 30);

    let verify_resp = send_http(
        addr,
        "POST",
        "/api/v1/auth/2fa/verify",
        &[],
        Some(&json!({
            "usuario_id": redator_id,
            "codigo_totp": codigo_totp,
            "desafio_2fa_token": desafio_2fa_token
        }).to_string()),
    )
    .await;
    assert_eq!(verify_resp.status, StatusCode::OK.as_u16());
    let set_cookie = verify_resp.header("set-cookie").expect("Set-Cookie presente");
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Strict"));
    let redator_cookie = extrair_cookie_sessao(&verify_resp).expect("cookie redator válido");

    // 3. Consulta /auth/me e auditoria de sessões ativas (zero dependência de ID no cliente)
    let me_resp = send_http(
        addr,
        "GET",
        "/api/v1/auth/me",
        &[("Cookie", &redator_cookie)],
        None,
    )
    .await;
    assert_eq!(me_resp.status, StatusCode::OK.as_u16());
    let me_json = me_resp.json();
    assert_eq!(me_json["autenticado"].as_bool(), Some(true));
    assert_eq!(me_json["totp_habilitado"].as_bool(), Some(true));
    assert_eq!(me_json["cargo_slug"].as_str(), Some("redator_municipal"));
    assert_eq!(me_json["escopo"].as_str(), Some("MUNICIPAL"));
    assert_eq!(me_json["ibge_codigo"].as_str(), Some("3550308"));
    assert_eq!(
        me_json["permissoes"]["pode_publicar_municipal"].as_bool(),
        Some(true)
    );

    let sessoes_resp = send_http(
        addr,
        "GET",
        "/api/v1/auth/sessoes",
        &[("Cookie", &redator_cookie)],
        None,
    )
    .await;
    assert_eq!(sessoes_resp.status, StatusCode::OK.as_u16());
    let sessoes_json = sessoes_resp.json();
    assert_eq!(sessoes_json["total_sessoes"].as_u64(), Some(1));
    assert_eq!(sessoes_json["sessoes"][0]["sessao_atual"].as_bool(), Some(true));

    // 4. Criação de notícia com bloqueio XSS (Ammonia) e consulta pública
    let criar_resp = send_http(
        addr,
        "POST",
        "/api/v1/noticias",
        &[("Cookie", &redator_cookie)],
        Some(
            &json!({
                "titulo": format!("Pauta E2E Auditada {}", Utc::now().timestamp()),
                "escopo": "MUNICIPAL",
                "ibge_codigo": "3550308",
                "conteudo_html": "<script>alert('xss')</script><p>Conteúdo legítimo federado.</p>",
                "publicar_agora": true
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(criar_resp.status, StatusCode::OK.as_u16());
    let criar_json = criar_resp.json();
    let noticia_id = criar_json["noticia_id"].as_str().expect("noticia_id").to_string();
    let slug = criar_json["slug"].as_str().expect("slug").to_string();
    let html_limpo = criar_json["conteudo_sanitizado"].as_str().expect("html limpo");
    assert!(!html_limpo.contains("<script"));
    assert!(html_limpo.contains("Conteúdo legítimo federado."));

    let detalhe_resp = send_http(addr, "GET", &format!("/api/v1/noticias/{}", slug), &[], None).await;
    assert_eq!(detalhe_resp.status, StatusCode::OK.as_u16());
    let detalhe_json = detalhe_resp.json();
    assert_eq!(detalhe_json["noticia"]["uf"].as_str(), Some("SP"));
    assert_eq!(
        detalhe_json["noticia"]["territorio_nome"].as_str(),
        Some("São Paulo")
    );

    let feed_resp = send_http(
        addr,
        "GET",
        "/api/v1/feed?uf=SP&q=Pauta%20E2E&limit=10&offset=0",
        &[],
        None,
    )
    .await;
    assert_eq!(feed_resp.status, StatusCode::OK.as_u16());
    assert_eq!(feed_resp.header("x-cache"), Some("MISS"));

    let feed_hit_resp = send_http(
        addr,
        "GET",
        "/api/v1/feed?uf=SP&q=Pauta%20E2E&limit=10&offset=0",
        &[],
        None,
    )
    .await;
    assert_eq!(feed_hit_resp.status, StatusCode::OK.as_u16());
    assert_eq!(feed_hit_resp.header("x-cache"), Some("HIT"));
    let feed_json = feed_resp.json();
    assert_eq!(feed_json["tem_mais"].as_bool(), Some(false));
    let feed_noticias = feed_json["noticias"].as_array().expect("lista noticias no feed");
    assert!(!feed_noticias.is_empty());
    assert_eq!(feed_noticias[0]["uf"].as_str(), Some("SP"));
    assert_eq!(
        feed_noticias[0]["territorio_nome"].as_str(),
        Some("São Paulo")
    );

    // 5. Votação imutável no ledger + rejeição de voto duplicado + quórum Hype >= 10
    let voto_path = format!("/api/v1/noticias/{}/voto", noticia_id);
    let primeiro_voto = send_http(
        addr,
        "POST",
        &voto_path,
        &[("Cookie", &redator_cookie)],
        Some(r#"{"tipo_voto":1}"#),
    )
    .await;
    assert_eq!(primeiro_voto.status, StatusCode::OK.as_u16());
    let primeiro_json = primeiro_voto.json();
    assert_eq!(primeiro_json["votos_positivos"].as_i64(), Some(1));
    assert_eq!(primeiro_json["hype_score"].as_f64(), Some(0.0));

    let voto_duplicado = send_http(
        addr,
        "POST",
        &voto_path,
        &[("Cookie", &redator_cookie)],
        Some(r#"{"tipo_voto":1}"#),
    )
    .await;
    assert_eq!(voto_duplicado.status, StatusCode::BAD_REQUEST.as_u16());

    let mut ultimo_score = 0.0f64;
    for i in 2i64..=10 {
        let filiado_id = Uuid::new_v4();
        let filiado_email = format!("filiado.{}.{}@jornalfederado.org.br", i, filiado_id.simple());
        sqlx::query(
            r#"
            INSERT INTO usuarios (
                id, nome, email, senha_hash, cargo_id, escopo, ibge_codigo,
                filiado_verificado, status
            )
            VALUES ($1, $2, $3, $4, 8, 'MUNICIPAL'::tipo_escopo, '3550308', TRUE, 'ATIVO'::status_usuario)
            "#,
        )
        .bind(filiado_id)
        .bind(format!("Filiado E2E {}", i))
        .bind(&filiado_email)
        .bind(&senha_hash)
        .execute(&pool)
        .await
        .expect("inserir filiado votante");

        let token = SessionManager::criar_sessao(
            &pool,
            filiado_id,
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "E2E-Test/1.0",
        )
        .await
        .expect("sessao filiado criada");
        let cookie_filiado = format!("jornal_session={}", token);

        let voto_resp = send_http(
            addr,
            "POST",
            &voto_path,
            &[("Cookie", &cookie_filiado)],
            Some(r#"{"tipo_voto":1}"#),
        )
        .await;
        assert_eq!(voto_resp.status, StatusCode::OK.as_u16());
        let voto_json = voto_resp.json();
        assert_eq!(voto_json["votos_positivos"].as_i64(), Some(i));
        ultimo_score = voto_json["hype_score"].as_f64().unwrap_or(0.0);
    }
    assert!(
        ultimo_score > 0.0,
        "Hype Score deve ficar > 0 após atingir quórum de 10 votos"
    );

    // 6. Pacote A: Acervo editorial + Escalonamento real (MUNICIPAL -> ESTADUAL) + Aprovação
    let acervo_resp = send_http(
        addr,
        "GET",
        "/api/v1/noticias/acervo",
        &[("Cookie", &redator_cookie)],
        None,
    )
    .await;
    assert_eq!(acervo_resp.status, StatusCode::OK.as_u16());
    let acervo_json = acervo_resp.json();
    assert!(acervo_json["noticias"]
        .as_array()
        .expect("lista acervo")
        .iter()
        .any(|n| n["id"].as_str() == Some(noticia_id.as_str())));

    let solicitar_esc_resp = send_http(
        addr,
        "POST",
        &format!("/api/v1/noticias/{}/escalonar", noticia_id),
        &[("Cookie", &redator_cookie)],
        Some(
            &json!({
                "escopo_origem": "MUNICIPAL",
                "escopo_destino": "ESTADUAL",
                "justificativa": "Alto engajamento comunitário verificado na pauta E2E."
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(solicitar_esc_resp.status, StatusCode::OK.as_u16());
    let solicitacao_id = solicitar_esc_resp.json()["solicitacao_id"]
        .as_str()
        .expect("solicitacao_id retornado")
        .to_string();

    // Cria liderança nacional com permissão de aprovar escalonamento (cargo_id = 2: lider_nacional)
    let lider_nac_id = Uuid::new_v4();
    let lider_nac_email = format!("lider.nac.{}@jornalfederado.org.br", lider_nac_id.simple());
    sqlx::query(
        r#"
        INSERT INTO usuarios (
            id, nome, email, senha_hash, cargo_id, escopo, ibge_codigo,
            filiado_verificado, status, totp_habilitado
        )
        VALUES ($1, 'Líder Nacional E2E', $2, $3, 2, 'NACIONAL'::tipo_escopo, NULL, TRUE, 'ATIVO'::status_usuario, FALSE)
        "#,
    )
    .bind(lider_nac_id)
    .bind(&lider_nac_email)
    .bind(&senha_hash)
    .execute(&pool)
    .await
    .expect("inserir líder nacional");

    let lider_nac_token = SessionManager::criar_sessao(
        &pool,
        lider_nac_id,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        "E2E-Lider/1.0",
    )
    .await
    .expect("sessão líder nacional");
    let lider_nac_cookie = format!("jornal_session={}", lider_nac_token);

    let fila_resp = send_http(
        addr,
        "GET",
        "/api/v1/escalonamentos?status=PENDENTE",
        &[("Cookie", &lider_nac_cookie)],
        None,
    )
    .await;
    assert_eq!(fila_resp.status, StatusCode::OK.as_u16());
    assert!(fila_resp.json()["solicitacoes"]
        .as_array()
        .expect("fila escalonamentos")
        .iter()
        .any(|s| s["id"].as_str() == Some(solicitacao_id.as_str())));

    let avaliar_resp = send_http(
        addr,
        "POST",
        &format!("/api/v1/escalonamentos/{}/avaliar", solicitacao_id),
        &[("Cookie", &lider_nac_cookie)],
        Some(
            &json!({
                "aprovado": true,
                "parecer_revisao": "Relevância estadual comprovada."
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(avaliar_resp.status, StatusCode::OK.as_u16());
    let avaliar_json = avaliar_resp.json();
    assert_eq!(avaliar_json["novo_escopo"].as_str(), Some("ESTADUAL"));
    assert_eq!(avaliar_json["novo_ibge_codigo"].as_str(), Some("35"));

    // 7. Pacote A: Interruptor Modo Eleitoral (Lei nº 9.504/97) congela Hype no Feed e depois restaura
    let ativar_eleitoral = send_http(
        addr,
        "POST",
        "/api/v1/infra/modo-eleitoral",
        &[("Cookie", &admin_cookie)],
        Some(r#"{"ativo":true,"motivo":"Teste E2E de congelamento eleitoral Lei 9504"}"#),
    )
    .await;
    assert_eq!(ativar_eleitoral.status, StatusCode::OK.as_u16());
    assert_eq!(
        ativar_eleitoral.json()["config"]["ativo"].as_bool(),
        Some(true)
    );

    let feed_eleitoral = send_http(addr, "GET", "/api/v1/feed?uf=SP&limit=5", &[], None).await;
    assert_eq!(feed_eleitoral.status, StatusCode::OK.as_u16());
    assert_eq!(feed_eleitoral.header("x-cache"), Some("MISS"));
    let feed_el_json = feed_eleitoral.json();
    assert_eq!(feed_el_json["modo_eleitoral_ativo"].as_bool(), Some(true));
    assert!(feed_el_json["noticias"][0]["hype_score"].is_null());

    let desativar_eleitoral = send_http(
        addr,
        "POST",
        "/api/v1/infra/modo-eleitoral",
        &[("Cookie", &admin_cookie)],
        Some(r#"{"ativo":false,"motivo":"Retorno à operação ordinária pós-teste E2E"}"#),
    )
    .await;
    assert_eq!(desativar_eleitoral.status, StatusCode::OK.as_u16());

    // 8. Pacote A: Provisionamento 2FA TOTP (setup + confirmar) e Painel de Auditoria
    let setup_2fa_resp = send_http(
        addr,
        "POST",
        "/api/v1/auth/2fa/setup",
        &[("Cookie", &lider_nac_cookie)],
        None,
    )
    .await;
    assert_eq!(setup_2fa_resp.status, StatusCode::OK.as_u16());
    let setup_2fa_json = setup_2fa_resp.json();
    let secret_b32 = setup_2fa_json["secret_base32"]
        .as_str()
        .expect("secret_base32 retornado");
    let secret_bytes = decodificar_base32_teste(secret_b32);
    assert_eq!(secret_bytes.len(), 20);

    let agora_setup = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("relogio valido")
        .as_secs();
    let codigo_confirmacao = CryptoService::gerar_totp_sha256(&secret_bytes, agora_setup / 30);

    let confirmar_2fa_resp = send_http(
        addr,
        "POST",
        "/api/v1/auth/2fa/confirmar",
        &[("Cookie", &lider_nac_cookie)],
        Some(&json!({ "codigo_totp": codigo_confirmacao }).to_string()),
    )
    .await;
    assert_eq!(confirmar_2fa_resp.status, StatusCode::OK.as_u16());
    let confirmar_json = confirmar_2fa_resp.json();
    assert_eq!(
        confirmar_json["recovery_codes"]
            .as_array()
            .expect("recovery_codes")
            .len(),
        8
    );

    let auditoria_resp = send_http(
        addr,
        "GET",
        "/api/v1/infra/auditoria",
        &[("Cookie", &admin_cookie)],
        None,
    )
    .await;
    assert_eq!(auditoria_resp.status, StatusCode::OK.as_u16());
    let auditoria_json = auditoria_resp.json();
    assert_eq!(auditoria_json["visao_global"].as_bool(), Some(true));
    assert!(!auditoria_json["logs"]
        .as_array()
        .expect("logs auditoria")
        .is_empty());

    // 9. Revogação global de sessões e validação de logs de auditoria
    let revogar_resp = send_http(
        addr,
        "POST",
        "/api/v1/auth/sessoes/revogar-todas",
        &[("Cookie", &redator_cookie)],
        Some(r#"{"manter_sessao_atual":false}"#),
    )
    .await;
    assert_eq!(revogar_resp.status, StatusCode::OK.as_u16());

    let apos_revogacao = send_http(
        addr,
        "GET",
        "/api/v1/auth/sessoes",
        &[("Cookie", &redator_cookie)],
        None,
    )
    .await;
    assert_eq!(apos_revogacao.status, StatusCode::UNAUTHORIZED.as_u16());

    let audit_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM auditoria_logs
        WHERE acao IN (
            'ROTACAO_CHAVE_AES_TOTP',
            'VERIFICACAO_2FA_SUCESSO',
            'ESCALONAMENTO_APROVADO',
            'MODO_ELEITORAL_ALTERADO',
            'ATIVACAO_2FA_TOTP',
            'REVOGACAO_TODAS_SESSOES'
        )
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("consultar auditoria_logs");
    assert!(audit_count >= 6);
}
