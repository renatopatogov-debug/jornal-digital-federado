use crate::config::AppConfig;
use crate::error::AppError;
use crate::middleware::rbac::{extrair_ip_cliente, extrair_token_cookie, AuthContext};
use crate::models::{StatusUsuario, TipoEscopo};
use crate::services::crypto::CryptoService;
use crate::services::session::{AuditoriaEvento, SessionManager};
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub senha: String,
}

#[derive(Debug, Deserialize)]
pub struct Verify2faRequest {
    pub usuario_id: Uuid,
    pub codigo_totp: String,
    pub desafio_2fa_token: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct RevogarSessoesRequest {
    pub manter_sessao_atual: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Confirmar2faRequest {
    pub codigo_totp: String,
}

#[derive(Debug, Serialize)]
pub struct Setup2faResponse {
    pub sucesso: bool,
    pub secret_base32: String,
    pub otpauth_url: String,
    pub algoritmo: &'static str,
    pub digitos: u8,
    pub periodo: u8,
    pub mensagem: String,
}

#[derive(Debug, Serialize)]
pub struct Confirmar2faResponse {
    pub sucesso: bool,
    pub recovery_codes: Vec<String>,
    pub mensagem: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub sucesso: bool,
    pub requer_2fa: bool,
    pub usuario_id: Option<Uuid>,
    pub desafio_2fa_token: Option<String>,
    pub mensagem: String,
}

#[derive(Debug, Serialize)]
pub struct MePermissoes {
    pub pode_gerenciar_infra: bool,
    pub pode_alterar_cargos: bool,
    pub pode_ativar_filiados: bool,
    pub pode_revogar_lote: bool,
    pub pode_publicar_nacional: bool,
    pub pode_publicar_estadual: bool,
    pub pode_publicar_municipal: bool,
    pub pode_solicitar_escalonamento: bool,
    pub pode_aprovar_escalonamento: bool,
    pub pode_votar_hype: bool,
    pub limite_ativacao_semanal: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub autenticado: bool,
    pub usuario_id: Uuid,
    pub nome: String,
    pub email: String,
    pub escopo: TipoEscopo,
    pub ibge_codigo: Option<String>,
    pub filiado_verificado: bool,
    pub totp_habilitado: bool,
    pub status: StatusUsuario,
    pub cargo_slug: String,
    pub cargo_nome: String,
    pub nivel_hierarquico: i32,
    pub permissoes: MePermissoes,
}

fn construir_cookie_sessao(token: &str, max_age_secs: i64) -> Option<HeaderValue> {
    let same_site = AppConfig::cookie_same_site_from_env();
    let secure = AppConfig::cookie_secure_from_env() || same_site == "None";
    let cookie = if secure {
        format!(
            "__Host-jornal_session={}; Path=/; Secure; HttpOnly; SameSite={}; Max-Age={}",
            token, same_site, max_age_secs
        )
    } else {
        format!(
            "jornal_session={}; Path=/; HttpOnly; SameSite={}; Max-Age={}",
            token, same_site, max_age_secs
        )
    };
    HeaderValue::from_str(&cookie).ok()
}

fn extrair_user_agent(headers: &HeaderMap) -> &str {
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("JornalClient/2.1")
}

async fn emitir_sessao_e_headers(
    pool: &PgPool,
    usuario_id: Uuid,
    ip: IpAddr,
    user_agent: &str,
) -> Result<HeaderMap, AppError> {
    let token = SessionManager::criar_sessao(pool, usuario_id, ip, user_agent).await?;
    let mut response_headers = HeaderMap::new();
    if let Some(cookie_val) = construir_cookie_sessao(&token, 604_800) {
        response_headers.insert(header::SET_COOKIE, cookie_val);
    }
    Ok(response_headers)
}

pub async fn login(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.email.trim().is_empty() || payload.senha.trim().is_empty() {
        return Err(AppError::BadRequest(
            "E-mail e senha são obrigatórios.".to_string(),
        ));
    }

    let ip = extrair_ip_cliente(&headers);
    let user_agent = extrair_user_agent(&headers);

    // Busca usuário e valida senha_hash em tempo constante
    let row = sqlx::query(
        r#"
        SELECT id, senha_hash, totp_habilitado, status::text as status
        FROM usuarios
        WHERE LOWER(email) = LOWER($1)
        "#,
    )
    .bind(payload.email.trim())
    .fetch_optional(&pool)
    .await?;

    let Some(row) = row else {
        SessionManager::registrar_auditoria(
            &pool,
            AuditoriaEvento {
                ator_id: None,
                acao: "LOGIN_FALHA",
                entidade: "sessoes",
                entidade_id: None,
                dados_novos: json!({ "motivo": "usuario_nao_encontrado" }),
                ip_origem: ip,
                user_agent: Some(user_agent),
            },
        )
        .await;
        return Err(AppError::Unauthorized("Credenciais inválidas.".to_string()));
    };

    let usuario_id: Uuid = row.try_get("id")?;
    let senha_hash: String = row.try_get("senha_hash")?;
    let totp_habilitado: bool = row.try_get("totp_habilitado")?;
    let status: String = row.try_get("status")?;

    if !CryptoService::verificar_senha_hash(&payload.senha, &senha_hash) {
        SessionManager::registrar_auditoria(
            &pool,
            AuditoriaEvento {
                ator_id: Some(usuario_id),
                acao: "LOGIN_FALHA",
                entidade: "sessoes",
                entidade_id: None,
                dados_novos: json!({ "motivo": "senha_invalida" }),
                ip_origem: ip,
                user_agent: Some(user_agent),
            },
        )
        .await;
        return Err(AppError::Unauthorized("Credenciais inválidas.".to_string()));
    }

    if status != "ATIVO" {
        return Err(AppError::Forbidden(
            "Usuário suspenso ou pendente de ativação pelo líder diretorial.".to_string(),
        ));
    }

    // Se 2FA estiver ativo, requer etapa subsequente de desafio TOTP ou código de recuperação
    if totp_habilitado {
        let agora = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .as_secs();
        let desafio_2fa_token = CryptoService::emitir_token_desafio_2fa(usuario_id, agora);

        SessionManager::registrar_auditoria(
            &pool,
            AuditoriaEvento {
                ator_id: Some(usuario_id),
                acao: "LOGIN_DESAFIO_2FA",
                entidade: "sessoes",
                entidade_id: None,
                dados_novos: json!({ "requer_2fa": true }),
                ip_origem: ip,
                user_agent: Some(user_agent),
            },
        )
        .await;

        return Ok((
            HeaderMap::new(),
            Json(LoginResponse {
                sucesso: true,
                requer_2fa: true,
                usuario_id: Some(usuario_id),
                desafio_2fa_token: Some(desafio_2fa_token),
                mensagem:
                    "Credencial validada. Confirme o segundo fator (TOTP ou código de recuperação)."
                        .to_string(),
            }),
        ));
    }

    let response_headers = emitir_sessao_e_headers(&pool, usuario_id, ip, user_agent).await?;
    let uid_str = usuario_id.to_string();
    SessionManager::registrar_auditoria(
        &pool,
        AuditoriaEvento {
            ator_id: Some(usuario_id),
            acao: "LOGIN_SUCESSO",
            entidade: "sessoes",
            entidade_id: Some(&uid_str),
            dados_novos: json!({ "requer_2fa": false }),
            ip_origem: ip,
            user_agent: Some(user_agent),
        },
    )
    .await;

    Ok((
        response_headers,
        Json(LoginResponse {
            sucesso: true,
            requer_2fa: false,
            usuario_id: Some(usuario_id),
            desafio_2fa_token: None,
            mensagem: "Login realizado e sessão HTTP-Only emitida com sucesso.".to_string(),
        }),
    ))
}

pub async fn verify_2fa(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<Verify2faRequest>,
) -> Result<impl IntoResponse, AppError> {
    let codigo = payload.codigo_totp.trim();
    if codigo.is_empty() {
        return Err(AppError::BadRequest(
            "Informe o código TOTP de 6 dígitos ou código de recuperação.".to_string(),
        ));
    }

    let agora = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        .as_secs();

    if !CryptoService::validar_token_desafio_2fa(
        &payload.desafio_2fa_token,
        payload.usuario_id,
        agora,
    ) {
        return Err(AppError::Unauthorized(
            "Desafio de pré-autenticação 2FA inválido ou expirado. Realize o login novamente."
                .to_string(),
        ));
    }

    let ip = extrair_ip_cliente(&headers);
    let user_agent = extrair_user_agent(&headers);

    let row = sqlx::query(
        r#"
        SELECT totp_habilitado,
               totp_secret_cifrado,
               COALESCE(recovery_codes_hashes, '{}') as recovery_codes_hashes,
               status::text as status
        FROM usuarios
        WHERE id = $1
        "#,
    )
    .bind(payload.usuario_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Usuário não encontrado.".to_string()))?;

    let status: String = row.try_get("status")?;
    let totp_habilitado: bool = row.try_get("totp_habilitado")?;
    let totp_secret_cifrado: Option<Vec<u8>> = row.try_get("totp_secret_cifrado")?;
    let recovery_codes_hashes: Vec<String> = row.try_get("recovery_codes_hashes")?;

    if status != "ATIVO" {
        return Err(AppError::Forbidden(
            "Usuário suspenso ou pendente de ativação.".to_string(),
        ));
    }
    if !totp_habilitado {
        return Err(AppError::BadRequest(
            "Segundo fator 2FA não está habilitado para esta conta.".to_string(),
        ));
    }

    let mut verificado = false;
    let mut metodo_usado = "totp";

    if let Some(envelope) = totp_secret_cifrado {
        if codigo.len() == 6 && codigo.chars().all(|c| c.is_ascii_digit()) {
            let crypto = CryptoService::from_env();
            let crypto_anterior = CryptoService::previous_from_env();
            let (secret, usou_chave_antiga) = crypto
                .decifrar_com_fallback(&envelope, crypto_anterior.as_ref())
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
            if CryptoService::verificar_totp_sha256(&secret, codigo, agora) {
                verificado = true;
                // Re-cifra preguiçosamente caso ainda estivesse sob chave mestra anterior
                if let Some(novo_envelope) = usou_chave_antiga
                    .then(|| crypto.cifrar_envelope_aleatorio(&secret))
                    .and_then(Result::ok)
                {
                    let _ = sqlx::query(
                        "UPDATE usuarios SET totp_secret_cifrado = $1 WHERE id = $2",
                    )
                    .bind(novo_envelope)
                    .bind(payload.usuario_id)
                    .execute(&pool)
                    .await;
                }
            }
        }
    }

    if !verificado && !recovery_codes_hashes.is_empty() {
        let codigo_hash = CryptoService::hash_token_sha256(codigo);
        if recovery_codes_hashes.iter().any(|h| h == &codigo_hash) {
            sqlx::query(
                r#"
                UPDATE usuarios
                SET recovery_codes_hashes = array_remove(recovery_codes_hashes, $1)
                WHERE id = $2
                "#,
            )
            .bind(&codigo_hash)
            .bind(payload.usuario_id)
            .execute(&pool)
            .await?;
            verificado = true;
            metodo_usado = "recovery_code";
        }
    }

    if !verificado {
        SessionManager::registrar_auditoria(
            &pool,
            AuditoriaEvento {
                ator_id: Some(payload.usuario_id),
                acao: "VERIFICACAO_2FA_FALHA",
                entidade: "sessoes",
                entidade_id: None,
                dados_novos: json!({ "motivo": "codigo_invalido" }),
                ip_origem: ip,
                user_agent: Some(user_agent),
            },
        )
        .await;
        return Err(AppError::Unauthorized(
            "Código TOTP ou código de recuperação inválido.".to_string(),
        ));
    }

    let response_headers = emitir_sessao_e_headers(&pool, payload.usuario_id, ip, user_agent).await?;
    let uid_str = payload.usuario_id.to_string();
    SessionManager::registrar_auditoria(
        &pool,
        AuditoriaEvento {
            ator_id: Some(payload.usuario_id),
            acao: "VERIFICACAO_2FA_SUCESSO",
            entidade: "sessoes",
            entidade_id: Some(&uid_str),
            dados_novos: json!({ "metodo": metodo_usado }),
            ip_origem: ip,
            user_agent: Some(user_agent),
        },
    )
    .await;

    Ok((
        response_headers,
        Json(json!({
            "sucesso": true,
            "usuario_id": payload.usuario_id,
            "mensagem": "Segundo fator autenticado e cookie HTTP-Only emitido."
        })),
    ))
}

pub async fn logout(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let ip = extrair_ip_cliente(&headers);
    let user_agent = extrair_user_agent(&headers);

    if let Some(token) = extrair_token_cookie(&headers) {
        if let Ok(Some(uid)) = SessionManager::revogar_sessao(&pool, &token).await {
            let uid_str = uid.to_string();
            SessionManager::registrar_auditoria(
                &pool,
                AuditoriaEvento {
                    ator_id: Some(uid),
                    acao: "LOGOUT_SUCESSO",
                    entidade: "sessoes",
                    entidade_id: Some(&uid_str),
                    dados_novos: json!({ "revogada": true }),
                    ip_origem: ip,
                    user_agent: Some(user_agent),
                },
            )
            .await;
        }
    }

    let mut response_headers = HeaderMap::new();
    if let Some(cookie_val) = construir_cookie_sessao("", 0) {
        response_headers.insert(header::SET_COOKIE, cookie_val);
    }

    Ok((
        response_headers,
        Json(json!({ "sucesso": true, "mensagem": "Sessão encerrada com sucesso." })),
    ))
}

pub async fn listar_sessoes(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token_atual = extrair_token_cookie(&headers);
    let sessoes =
        SessionManager::listar_sessoes_usuario(&pool, auth.usuario_id, token_atual.as_deref())
            .await?;

    Ok(Json(json!({
        "sucesso": true,
        "usuario_id": auth.usuario_id,
        "total_sessoes": sessoes.len(),
        "sessoes": sessoes
    })))
}

pub async fn revogar_todas_sessoes(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    let payload: RevogarSessoesRequest = if body.is_empty() {
        RevogarSessoesRequest::default()
    } else {
        serde_json::from_slice(&body).map_err(|_| {
            AppError::BadRequest("Corpo JSON inválido para revogação de sessões.".to_string())
        })?
    };

    let manter_atual = payload.manter_sessao_atual.unwrap_or(false);
    let token_atual = if manter_atual {
        extrair_token_cookie(&headers)
    } else {
        None
    };

    let revogadas =
        SessionManager::revogar_todas_sessoes(&pool, auth.usuario_id, token_atual.as_deref())
            .await?;

    let ip = extrair_ip_cliente(&headers);
    let user_agent = extrair_user_agent(&headers);
    let uid_str = auth.usuario_id.to_string();
    SessionManager::registrar_auditoria(
        &pool,
        AuditoriaEvento {
            ator_id: Some(auth.usuario_id),
            acao: "REVOGACAO_TODAS_SESSOES",
            entidade: "sessoes",
            entidade_id: Some(&uid_str),
            dados_novos: json!({
                "manter_sessao_atual": manter_atual,
                "sessoes_revogadas": revogadas
            }),
            ip_origem: ip,
            user_agent: Some(user_agent),
        },
    )
    .await;

    let mut response_headers = HeaderMap::new();
    if let Some(cookie_val) = (!manter_atual)
        .then(|| construir_cookie_sessao("", 0))
        .flatten()
    {
        response_headers.insert(header::SET_COOKIE, cookie_val);
    }

    Ok((
        response_headers,
        Json(json!({
            "sucesso": true,
            "sessoes_revogadas": revogadas,
            "mensagem": "Sessões revogadas e evento registrado em auditoria."
        })),
    ))
}

/// Inicia o provisionamento seguro de 2FA TOTP (etapa 1: cifra o segredo sem ativar `totp_habilitado`)
pub async fn setup_2fa(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
) -> Result<Json<Setup2faResponse>, AppError> {
    let ja_habilitado: bool =
        sqlx::query_scalar("SELECT totp_habilitado FROM usuarios WHERE id = $1")
            .bind(auth.usuario_id)
            .fetch_optional(&pool)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Usuário não encontrado.".to_string()))?;

    if ja_habilitado {
        return Err(AppError::BadRequest(
            "Segundo fator 2FA já está ativo nesta conta.".to_string(),
        ));
    }

    let mut secret = [0u8; 20];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut secret);

    let crypto = CryptoService::from_env();
    let envelope = crypto
        .cifrar_envelope_aleatorio(&secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    sqlx::query("UPDATE usuarios SET totp_secret_cifrado = $1 WHERE id = $2")
        .bind(envelope)
        .bind(auth.usuario_id)
        .execute(&pool)
        .await?;

    let secret_base32 = CryptoService::base32_encode(&secret);
    let otpauth_url = format!(
        "otpauth://totp/JornalFederado:{}?secret={}&issuer=JornalFederado&algorithm=SHA256&digits=6&period=30",
        auth.email, secret_base32
    );

    let ip = extrair_ip_cliente(&headers);
    let user_agent = extrair_user_agent(&headers);
    let uid_str = auth.usuario_id.to_string();
    SessionManager::registrar_auditoria(
        &pool,
        AuditoriaEvento {
            ator_id: Some(auth.usuario_id),
            acao: "SETUP_2FA_INICIADO",
            entidade: "usuarios",
            entidade_id: Some(&uid_str),
            dados_novos: json!({ "algoritmo": "SHA256", "digitos": 6, "periodo": 30 }),
            ip_origem: ip,
            user_agent: Some(user_agent),
        },
    )
    .await;

    Ok(Json(Setup2faResponse {
        sucesso: true,
        secret_base32,
        otpauth_url,
        algoritmo: "SHA256",
        digitos: 6,
        periodo: 30,
        mensagem: "Segredo TOTP gerado e cifrado em repouso. Confirme um código de 6 dígitos para ativar o 2FA."
            .to_string(),
    }))
}

/// Confirma um código TOTP de 6 dígitos (etapa 2), ativa `totp_habilitado` e emite 8 códigos de recuperação
pub async fn confirmar_2fa(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
    Json(payload): Json<Confirmar2faRequest>,
) -> Result<Json<Confirmar2faResponse>, AppError> {
    let codigo = payload.codigo_totp.trim();
    if codigo.len() != 6 || !codigo.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::BadRequest(
            "Informe um código TOTP numérico de exatamente 6 dígitos.".to_string(),
        ));
    }

    let row = sqlx::query(
        "SELECT totp_habilitado, totp_secret_cifrado FROM usuarios WHERE id = $1",
    )
    .bind(auth.usuario_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Usuário não encontrado.".to_string()))?;

    let totp_habilitado: bool = row.try_get("totp_habilitado")?;
    let totp_secret_cifrado: Option<Vec<u8>> = row.try_get("totp_secret_cifrado")?;

    if totp_habilitado {
        return Err(AppError::BadRequest(
            "Segundo fator 2FA já foi confirmado e ativado nesta conta.".to_string(),
        ));
    }

    let Some(envelope) = totp_secret_cifrado else {
        return Err(AppError::BadRequest(
            "Nenhum segredo TOTP pendente. Inicie o provisionamento em /api/v1/auth/2fa/setup primeiro."
                .to_string(),
        ));
    };

    let crypto = CryptoService::from_env();
    let crypto_anterior = CryptoService::previous_from_env();
    let (secret, _) = crypto
        .decifrar_com_fallback(&envelope, crypto_anterior.as_ref())
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let agora = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        .as_secs();

    if !CryptoService::verificar_totp_sha256(&secret, codigo, agora) {
        return Err(AppError::BadRequest(
            "Código TOTP inválido. Verifique o aplicativo autenticador (SHA-256 / 30s) e tente novamente."
                .to_string(),
        ));
    }

    let recovery_codes = CryptoService::gerar_codigos_recuperacao();
    let recovery_hashes: Vec<String> = recovery_codes
        .iter()
        .map(|c| CryptoService::hash_token_sha256(c))
        .collect();

    sqlx::query(
        r#"
        UPDATE usuarios
        SET totp_habilitado = TRUE,
            recovery_codes_hashes = $1
        WHERE id = $2
        "#,
    )
    .bind(&recovery_hashes)
    .bind(auth.usuario_id)
    .execute(&pool)
    .await?;

    let ip = extrair_ip_cliente(&headers);
    let user_agent = extrair_user_agent(&headers);
    let uid_str = auth.usuario_id.to_string();
    SessionManager::registrar_auditoria(
        &pool,
        AuditoriaEvento {
            ator_id: Some(auth.usuario_id),
            acao: "ATIVACAO_2FA_TOTP",
            entidade: "usuarios",
            entidade_id: Some(&uid_str),
            dados_novos: json!({
                "totp_habilitado": true,
                "recovery_codes_emitidos": recovery_codes.len()
            }),
            ip_origem: ip,
            user_agent: Some(user_agent),
        },
    )
    .await;

    Ok(Json(Confirmar2faResponse {
        sucesso: true,
        recovery_codes,
        mensagem:
            "Segundo fator 2FA TOTP ativado com sucesso! Guarde os códigos de recuperação em local seguro."
                .to_string(),
    }))
}

/// Retorna a identidade, cargo e permissões territoriais da sessão autenticada via cookie HTTP-Only
pub async fn me(auth: AuthContext) -> Json<MeResponse> {
    Json(MeResponse {
        autenticado: true,
        usuario_id: auth.usuario_id,
        nome: auth.nome,
        email: auth.email,
        escopo: auth.escopo,
        ibge_codigo: auth.ibge_codigo,
        filiado_verificado: auth.filiado_verificado,
        totp_habilitado: auth.totp_habilitado,
        status: auth.status,
        cargo_slug: auth.cargo.slug,
        cargo_nome: auth.cargo.nome,
        nivel_hierarquico: auth.cargo.nivel_hierarquico,
        permissoes: MePermissoes {
            pode_gerenciar_infra: auth.cargo.pode_gerenciar_infra,
            pode_alterar_cargos: auth.cargo.pode_alterar_cargos,
            pode_ativar_filiados: auth.cargo.pode_ativar_filiados,
            pode_revogar_lote: auth.cargo.pode_revogar_lote,
            pode_publicar_nacional: auth.cargo.pode_publicar_nacional,
            pode_publicar_estadual: auth.cargo.pode_publicar_estadual,
            pode_publicar_municipal: auth.cargo.pode_publicar_municipal,
            pode_solicitar_escalonamento: auth.cargo.pode_solicitar_escalonamento,
            pode_aprovar_escalonamento: auth.cargo.pode_aprovar_escalonamento,
            pode_votar_hype: auth.cargo.pode_votar_hype,
            limite_ativacao_semanal: auth.cargo.limite_ativacao_semanal,
        },
    })
}
