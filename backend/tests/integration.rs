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

fn config_teste(global_rpm: u32, auth_rpm: u32) -> AppConfig {
    AppConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        database_url: "postgres://postgres:postgres@localhost:5432/jornal_test".to_string(),
        redis_url: "redis://127.0.0.1:6379".to_string(),
        database_max_connections: 5,
        modo_eleitoral_default: false,
        max_ativacao_semanal_lider_municipal: 50,
        rate_limit_votos_por_minuto: 10,
        rate_limit_global_per_minute: global_rpm,
        rate_limit_auth_per_minute: auth_rpm,
        cors_allowed_origins: vec![
            "http://localhost:3000".to_string(),
            "http://127.0.0.1:3000".to_string(),
        ],
        cookie_secure: false,
        cookie_same_site: "Strict".to_string(),
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
        let _ = axum::serve(listener, app).await;
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

#[tokio::test]
async fn openapi_e_docs_respondem_com_especificacao_e_headers_seguranca() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
        .expect("pool lazy válido");
    let addr = spawn_server(pool, &config_teste(120, 15)).await;

    let spec_resp = send_http(addr, "GET", "/api/openapi.json", &[], None).await;
    assert_eq!(spec_resp.status, StatusCode::OK.as_u16());
    assert_eq!(spec_resp.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(spec_resp.header("x-frame-options"), Some("DENY"));

    let spec = spec_resp.json();
    assert_eq!(spec["openapi"].as_str(), Some("3.0.3"));
    assert!(spec["paths"]["/api/v1/auth/login"].is_object());
    assert!(spec["paths"]["/api/v1/auth/sessoes"].is_object());
    assert!(spec["paths"]["/api/v1/noticias/{id}/voto"].is_object());
    assert!(spec["paths"]["/api/v1/infra/rotacionar-chaves-totp"].is_object());

    let docs_resp = send_http(addr, "GET", "/api/docs", &[], None).await;
    assert_eq!(docs_resp.status, StatusCode::OK.as_u16());
    assert!(docs_resp.body.contains("/api/openapi.json"));
}

#[tokio::test]
async fn rotas_protegidas_exigem_cookie_de_sessao() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
        .expect("pool lazy válido");
    let addr = spawn_server(pool, &config_teste(120, 15)).await;

    let rotas = [
        ("GET", "/api/v1/auth/sessoes", None),
        ("POST", "/api/v1/auth/sessoes/revogar-todas", Some("{}")),
        (
            "POST",
            "/api/v1/noticias/preview-sanitizado",
            Some(r#"{"conteudo_html":"<p>oi</p>"}"#),
        ),
        (
            "POST",
            "/api/v1/infra/rotacionar-chaves-totp",
            Some("{}"),
        ),
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
    let addr = spawn_server(pool, &config_teste(120, 15)).await;

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
    let addr = spawn_server(pool, &config_teste(10, 3)).await;

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

    // Outro IP permanece liberado
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

    let addr = spawn_server(pool.clone(), &config_teste(500, 200)).await;

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
    assert_eq!(login_redator.json()["requer_2fa"].as_bool(), Some(true));

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
        Some(&json!({ "usuario_id": redator_id, "codigo_totp": codigo_totp }).to_string()),
    )
    .await;
    assert_eq!(verify_resp.status, StatusCode::OK.as_u16());
    let set_cookie = verify_resp.header("set-cookie").expect("Set-Cookie presente");
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Strict"));
    let redator_cookie = extrair_cookie_sessao(&verify_resp).expect("cookie redator válido");

    // 3. Auditoria de sessões ativas
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

    // 6. Revogação global de sessões e validação de logs de auditoria
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
        "SELECT COUNT(*) FROM auditoria_logs WHERE acao IN ('ROTACAO_CHAVE_AES_TOTP', 'VERIFICACAO_2FA_SUCESSO', 'REVOGACAO_TODAS_SESSOES')",
    )
    .fetch_one(&pool)
    .await
    .expect("consultar auditoria_logs");
    assert!(audit_count >= 3);
}
