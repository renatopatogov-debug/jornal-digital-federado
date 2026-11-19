use axum::{response::Html, Json};
use serde_json::{json, Value};

/// Especificação OpenAPI 3.0.3 da Core API do Jornal Digital Federado
pub fn construir_especificacao_openapi() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Jornal Digital Federado — Core API",
            "version": "2.1.0",
            "description": "API editorial federada com RBAC territorial IBGE (9 cargos), ledger eleitoral imutável (ADRs D1/D2), sanitização XSS estrita via Ammonia (ADR D8), Modo Eleitoral (Lei nº 9.504/97) e rotação de chaves AES-256-GCM."
        },
        "servers": [
            { "url": "/", "description": "Instância ativa" }
        ],
        "tags": [
            { "name": "Observabilidade", "description": "Saúde, métricas Prometheus e especificação OpenAPI" },
            { "name": "Feed Público", "description": "Leitura pública federada com suporte ao Modo Eleitoral" },
            { "name": "Autenticação & Sessões", "description": "Login, desafio 2FA TOTP/recuperação, cookies HTTP-Only e auditoria" },
            { "name": "Redação & Editorial", "description": "Preview Ammonia, publicação territorial e escalonamento" },
            { "name": "Votação Hype", "description": "Ledger imutável auditável de votos de filiados" },
            { "name": "Governança & Infraestrutura", "description": "Ativação de filiados, revogação em lote e rotação de chaves AES-256-GCM" }
        ],
        "components": {
            "securitySchemes": {
                "cookieAuth": {
                    "type": "apiKey",
                    "in": "cookie",
                    "name": "__Host-jornal_session",
                    "description": "Cookie HTTP-Only de sessão (em desenvolvimento sem TLS: jornal_session)."
                }
            },
            "schemas": {
                "TipoEscopo": { "type": "string", "enum": ["NACIONAL", "ESTADUAL", "MUNICIPAL"] },
                "StatusNoticia": { "type": "string", "enum": ["RASCUNHO", "EM_REVISAO", "PUBLICADO", "ARQUIVADO", "REJEITADO"] },
                "TipoFormatoMateria": { "type": "string", "enum": ["TEXTO", "VIDEO", "AUDIO", "IMPRESSO_PDF"] },
                "ErrorResponse": {
                    "type": "object",
                    "required": ["erro", "mensagem", "status_code"],
                    "properties": {
                        "erro": { "type": "boolean" },
                        "mensagem": { "type": "string" },
                        "status_code": { "type": "integer" }
                    }
                },
                "LoginRequest": {
                    "type": "object",
                    "required": ["email", "senha"],
                    "properties": {
                        "email": { "type": "string", "format": "email" },
                        "senha": { "type": "string", "format": "password" }
                    }
                },
                "Verify2faRequest": {
                    "type": "object",
                    "required": ["usuario_id", "codigo_totp", "desafio_2fa_token"],
                    "properties": {
                        "usuario_id": { "type": "string", "format": "uuid" },
                        "codigo_totp": { "type": "string", "description": "Código TOTP de 6 dígitos ou código de recuperação" },
                        "desafio_2fa_token": { "type": "string", "description": "Token HMAC temporário (5 min) emitido por /api/v1/auth/login" }
                    }
                },
                "CriarNoticiaRequest": {
                    "type": "object",
                    "required": ["titulo", "escopo", "conteudo_html"],
                    "properties": {
                        "titulo": { "type": "string", "minLength": 5 },
                        "subtitulo": { "type": "string", "nullable": true },
                        "escopo": { "$ref": "#/components/schemas/TipoEscopo" },
                        "ibge_codigo": { "type": "string", "nullable": true },
                        "formato": { "$ref": "#/components/schemas/TipoFormatoMateria" },
                        "video_url": { "type": "string", "nullable": true },
                        "audio_url": { "type": "string", "nullable": true },
                        "pdf_edicao_url": { "type": "string", "nullable": true },
                        "conteudo_html": { "type": "string" },
                        "imagem_destaque_url": { "type": "string", "nullable": true },
                        "publicar_agora": { "type": "boolean" }
                    }
                },
                "SolicitarEscalonamentoRequest": {
                    "type": "object",
                    "required": ["escopo_origem", "escopo_destino", "justificativa"],
                    "properties": {
                        "escopo_origem": { "$ref": "#/components/schemas/TipoEscopo" },
                        "escopo_destino": { "$ref": "#/components/schemas/TipoEscopo" },
                        "justificativa": { "type": "string" }
                    }
                },
                "SubmeterVotoRequest": {
                    "type": "object",
                    "required": ["tipo_voto"],
                    "properties": {
                        "tipo_voto": { "type": "integer", "enum": [-1, 1] }
                    }
                },
                "AtivarFiliadoRequest": {
                    "type": "object",
                    "properties": {
                        "filiado_id": { "type": "string", "format": "uuid", "nullable": true },
                        "email": { "type": "string", "format": "email", "nullable": true }
                    }
                },
                "RevogarLoteRequest": {
                    "type": "object",
                    "required": ["motivo"],
                    "properties": {
                        "lider_dissidente_id": { "type": "string", "format": "uuid", "nullable": true },
                        "lider_dissidente_email": { "type": "string", "format": "email", "nullable": true },
                        "motivo": { "type": "string", "minLength": 5 }
                    }
                },
                "RotacionarChavesRequest": {
                    "type": "object",
                    "properties": {
                        "chave_anterior": { "type": "string", "nullable": true }
                    }
                }
            }
        },
        "paths": {
            "/health": {
                "get": {
                    "tags": ["Observabilidade"],
                    "summary": "Health check da API e conectividade PostgreSQL 16",
                    "responses": { "200": { "description": "Serviço saudável" } }
                }
            },
            "/metrics": {
                "get": {
                    "tags": ["Observabilidade"],
                    "summary": "Métricas OpenMetrics/Prometheus",
                    "responses": { "200": { "description": "Texto plano Prometheus" } }
                }
            },
            "/api/openapi.json": {
                "get": {
                    "tags": ["Observabilidade"],
                    "summary": "Especificação OpenAPI 3.0.3 em JSON",
                    "responses": { "200": { "description": "Documento OpenAPI 3.0.3" } }
                }
            },
            "/api/docs": {
                "get": {
                    "tags": ["Observabilidade"],
                    "summary": "Portal HTML interativo da documentação da API",
                    "responses": { "200": { "description": "Página HTML da documentação" } }
                }
            },
            "/api/v1/feed": {
                "get": {
                    "tags": ["Feed Público"],
                    "summary": "Lista matérias publicadas com filtro UF/IBGE/busca/formato",
                    "parameters": [
                        { "name": "uf", "in": "query", "schema": { "type": "string" } },
                        { "name": "ibge", "in": "query", "schema": { "type": "string" } },
                        { "name": "q", "in": "query", "schema": { "type": "string" } },
                        { "name": "formato", "in": "query", "schema": { "$ref": "#/components/schemas/TipoFormatoMateria" } },
                        { "name": "limit", "in": "query", "schema": { "type": "integer", "minimum": 1, "maximum": 50 } },
                        { "name": "offset", "in": "query", "schema": { "type": "integer", "minimum": 0 } },
                        { "name": "incluir_nacional", "in": "query", "schema": { "type": "boolean" } }
                    ],
                    "responses": { "200": { "description": "Feed editorial com paginação e estado do Modo Eleitoral" } }
                }
            },
            "/api/v1/auth/login": {
                "post": {
                    "tags": ["Autenticação & Sessões"],
                    "summary": "Autentica credenciais e inicia desafio 2FA ou emite cookie HTTP-Only",
                    "requestBody": {
                        "required": true,
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/LoginRequest" } } }
                    },
                    "responses": {
                        "200": { "description": "Autenticado ou aguardando 2FA" },
                        "401": { "description": "Credenciais inválidas" },
                        "429": { "description": "Rate limit de autenticação excedido" }
                    }
                }
            },
            "/api/v1/auth/2fa/verify": {
                "post": {
                    "tags": ["Autenticação & Sessões"],
                    "summary": "Valida código TOTP SHA-256 ou código de recuperação e emite cookie de sessão",
                    "requestBody": {
                        "required": true,
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Verify2faRequest" } } }
                    },
                    "responses": {
                        "200": { "description": "Cookie HTTP-Only emitido" },
                        "401": { "description": "Código inválido" },
                        "429": { "description": "Rate limit de autenticação excedido" }
                    }
                }
            },
            "/api/v1/auth/logout": {
                "post": {
                    "tags": ["Autenticação & Sessões"],
                    "summary": "Revoga a sessão corrente e limpa o cookie",
                    "responses": { "200": { "description": "Sessão encerrada" } }
                }
            },
            "/api/v1/auth/me": {
                "get": {
                    "tags": ["Autenticação & Sessões"],
                    "summary": "Retorna perfil, cargo RBAC, território IBGE e permissões da sessão autenticada",
                    "security": [{ "cookieAuth": [] }],
                    "responses": {
                        "200": { "description": "Identidade autenticada" },
                        "401": { "description": "Não autenticado" }
                    }
                }
            },
            "/api/v1/auth/sessoes": {
                "get": {
                    "tags": ["Autenticação & Sessões"],
                    "summary": "Lista sessões ativas do usuário autenticado (IP, User-Agent e timestamps)",
                    "security": [{ "cookieAuth": [] }],
                    "responses": {
                        "200": { "description": "Lista de sessões ativas" },
                        "401": { "description": "Não autenticado" }
                    }
                }
            },
            "/api/v1/auth/sessoes/revogar-todas": {
                "post": {
                    "tags": ["Autenticação & Sessões"],
                    "summary": "Revoga todas as sessões do usuário autenticado (opcional manter_sessao_atual)",
                    "security": [{ "cookieAuth": [] }],
                    "requestBody": {
                        "required": false,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": { "manter_sessao_atual": { "type": "boolean" } }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Sessões revogadas" },
                        "401": { "description": "Não autenticado" }
                    }
                }
            },
            "/api/v1/noticias": {
                "post": {
                    "tags": ["Redação & Editorial"],
                    "summary": "Cria e publica/salva matéria com sanitização Ammonia e isolamento territorial",
                    "security": [{ "cookieAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CriarNoticiaRequest" } } }
                    },
                    "responses": {
                        "200": { "description": "Matéria registrada e sanitizada" },
                        "401": { "description": "Não autenticado" },
                        "403": { "description": "Violação RBAC ou territorial IBGE" }
                    }
                }
            },
            "/api/v1/noticias/preview-sanitizado": {
                "post": {
                    "tags": ["Redação & Editorial"],
                    "summary": "Executa pré-visualização segura via sanitizador Ammonia",
                    "security": [{ "cookieAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["conteudo_html"],
                                    "properties": { "conteudo_html": { "type": "string" } }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "HTML sanitizado retornado" },
                        "401": { "description": "Não autenticado" },
                        "403": { "description": "Sem permissão editorial" }
                    }
                }
            },
            "/api/v1/noticias/{id}": {
                "get": {
                    "tags": ["Feed Público"],
                    "summary": "Obtém detalhes completos de uma matéria publicada por slug",
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string" }, "description": "Slug da matéria" }
                    ],
                    "responses": {
                        "200": { "description": "Detalhes da matéria" },
                        "404": { "description": "Matéria não encontrada" }
                    }
                }
            },
            "/api/v1/noticias/{id}/escalonar": {
                "post": {
                    "tags": ["Redação & Editorial"],
                    "summary": "Solicita promoção editorial Municipal -> Estadual ou Estadual -> Nacional",
                    "security": [{ "cookieAuth": [] }],
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SolicitarEscalonamentoRequest" } } }
                    },
                    "responses": {
                        "200": { "description": "Solicitação registrada" },
                        "403": { "description": "Sem permissão ou fora da jurisdição" },
                        "404": { "description": "Matéria não encontrada" }
                    }
                }
            },
            "/api/v1/noticias/{id}/voto": {
                "post": {
                    "tags": ["Votação Hype"],
                    "summary": "Registra voto imutável (+1 ou -1) e recalcula Hype Score se qualificada",
                    "security": [{ "cookieAuth": [] }],
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SubmeterVotoRequest" } } }
                    },
                    "responses": {
                        "200": { "description": "Voto registrado no ledger" },
                        "400": { "description": "Voto duplicado ou valor inválido" },
                        "403": { "description": "Filiado não verificado" },
                        "404": { "description": "Matéria inexistente ou não publicada" },
                        "429": { "description": "Rate limit de votos excedido" }
                    }
                }
            },
            "/api/v1/usuarios/ativar": {
                "post": {
                    "tags": ["Governança & Infraestrutura"],
                    "summary": "Ativa filiado respeitando jurisdição IBGE e cota semanal de liderança municipal",
                    "security": [{ "cookieAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/AtivarFiliadoRequest" } } }
                    },
                    "responses": {
                        "200": { "description": "Filiado ativado" },
                        "403": { "description": "Sem privilégio ou fora da jurisdição" },
                        "429": { "description": "Cota semanal atingida" }
                    }
                }
            },
            "/api/v1/usuarios/revogar-lote": {
                "post": {
                    "tags": ["Governança & Infraestrutura"],
                    "summary": "Kill-Switch: suspende liderança dissidente, cadeia ativada por ela e expurga sessões",
                    "security": [{ "cookieAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/RevogarLoteRequest" } } }
                    },
                    "responses": {
                        "200": { "description": "Revogação executada" },
                        "403": { "description": "Sem permissão ou fora da jurisdição" }
                    }
                }
            },
            "/api/v1/infra/rotacionar-chaves-totp": {
                "post": {
                    "tags": ["Governança & Infraestrutura"],
                    "summary": "Re-cifra segredos 2FA AES-256-GCM da chave anterior para a chave mestra ativa",
                    "security": [{ "cookieAuth": [] }],
                    "requestBody": {
                        "required": false,
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/RotacionarChavesRequest" } } }
                    },
                    "responses": {
                        "200": { "description": "Rotação concluída em transação atômica" },
                        "401": { "description": "Não autenticado" },
                        "403": { "description": "Requer privilégio pode_gerenciar_infra (SuperAdmin)" }
                    }
                }
            }
        }
    })
}

pub async fn openapi_json() -> Json<Value> {
    Json(construir_especificacao_openapi())
}

pub async fn openapi_docs() -> Html<String> {
    Html(
        r###"<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Jornal Digital Federado — API Docs (OpenAPI 3.0.3)</title>
  <style>
    body { font-family: system-ui, -apple-system, sans-serif; margin: 0; background: #f7f2e4; color: #1f1b16; }
    header { background: #8b261d; color: #fff; padding: 20px 28px; }
    header h1 { margin: 0 0 6px; font-size: 1.4rem; }
    header p { margin: 0; opacity: 0.9; font-size: 0.92rem; }
    main { max-width: 980px; margin: 24px auto; padding: 0 20px 48px; }
    .meta { display: flex; gap: 12px; align-items: center; margin-bottom: 20px; flex-wrap: wrap; }
    .btn { display: inline-block; padding: 8px 14px; background: #1f1b16; color: #fff; text-decoration: none; border-radius: 6px; font-size: 0.88rem; }
    .card { background: #fff; border: 1px solid #dfd5c2; border-radius: 8px; padding: 14px 16px; margin-bottom: 12px; }
    .row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
    .method { font-weight: 700; font-size: 0.78rem; padding: 3px 8px; border-radius: 4px; text-transform: uppercase; color: #fff; }
    .get { background: #1d6f42; }
    .post { background: #1b4f9c; }
    .path { font-family: ui-monospace, SFMono-Regular, monospace; font-weight: 600; }
    .tag { font-size: 0.76rem; background: #efe8d8; color: #5b4d3c; padding: 2px 8px; border-radius: 999px; }
    .summary { margin: 8px 0 0; font-size: 0.92rem; color: #4a4033; }
  </style>
</head>
<body>
  <header>
    <h1>Jornal Digital Federado — Documentação da API v2.1.0</h1>
    <p>Especificação OpenAPI 3.0.3 (Auth HTTP-Only Cookie, RBAC IBGE, Ledger Hype e Infraestrutura AES-256-GCM)</p>
  </header>
  <main>
    <div class="meta">
      <a class="btn" href="/api/openapi.json">Baixar OpenAPI JSON (/api/openapi.json)</a>
      <span id="status">Carregando endpoints...</span>
    </div>
    <div id="endpoints"></div>
    <noscript>Ative JavaScript para visualizar a lista interativa ou acesse <a href="/api/openapi.json">/api/openapi.json</a>.</noscript>
  </main>
  <script>
    fetch('/api/openapi.json')
      .then(r => r.json())
      .then(spec => {
        const container = document.getElementById('endpoints');
        const entries = Object.entries(spec.paths || {});
        document.getElementById('status').textContent = entries.length + ' caminhos documentados';
        for (const [path, ops] of entries) {
          for (const [method, op] of Object.entries(ops)) {
            const card = document.createElement('div');
            card.className = 'card';
            const tag = (op.tags && op.tags[0]) || 'Geral';
            const sec = op.security ? ' · Requer Cookie HTTP-Only' : ' · Público';
            card.innerHTML =
              '<div class="row">' +
                '<span class="method ' + method.toLowerCase() + '">' + method + '</span>' +
                '<span class="path">' + path + '</span>' +
                '<span class="tag">' + tag + sec + '</span>' +
              '</div>' +
              '<p class="summary">' + (op.summary || '') + '</p>';
            container.appendChild(card);
          }
        }
      })
      .catch(() => {
        document.getElementById('status').textContent = 'Erro ao carregar /api/openapi.json';
      });
  </script>
</body>
</html>
"###
        .to_string(),
    )
}
