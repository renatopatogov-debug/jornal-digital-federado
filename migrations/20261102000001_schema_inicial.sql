-- =============================================================================
-- MIGRATION: 20261102000001_schema_inicial.sql
-- PROJETO: Jornal Digital Federado (Plano Diretor v2.1)
-- COMPATIBILIDADE: PostgreSQL 16+
-- =============================================================================

BEGIN;

-- 1. EXTENSÕES DO NÚCLEO
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "citext";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 2. DOMÍNIOS E TIPOS ENUMERADOS
DO $$ BEGIN
    CREATE TYPE tipo_escopo AS ENUM ('NACIONAL', 'ESTADUAL', 'MUNICIPAL');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE status_usuario AS ENUM ('ATIVO', 'SUSPENSO', 'PENDENTE');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE status_noticia AS ENUM ('RASCUNHO', 'EM_REVISAO', 'PUBLICADO', 'ARQUIVADO', 'REJEITADO');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE status_escalonamento AS ENUM ('PENDENTE', 'APROVADO', 'REJEITADO');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 3. TABELA: TERRITÓRIOS (Hierarquia Oficial IBGE)
-- ADR D4: Suporte a 27 Estados (código 2 dígitos) e 5.570 Municípios (código 7 dígitos)
CREATE TABLE IF NOT EXISTS territorios (
    codigo_ibge VARCHAR(7) PRIMARY KEY,
    nome VARCHAR(150) NOT NULL,
    tipo tipo_escopo NOT NULL CHECK (tipo IN ('ESTADUAL', 'MUNICIPAL')),
    uf CHAR(2) NOT NULL,
    ibge_uf_codigo VARCHAR(2) NOT NULL,
    populacao_estimada INT DEFAULT 0,
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_codigo_ibge_formato CHECK (
        (tipo = 'ESTADUAL' AND LENGTH(codigo_ibge) = 2) OR
        (tipo = 'MUNICIPAL' AND LENGTH(codigo_ibge) = 7)
    )
);

CREATE INDEX IF NOT EXISTS idx_territorios_uf ON territorios (uf);
CREATE INDEX IF NOT EXISTS idx_territorios_tipo ON territorios (tipo);
CREATE INDEX IF NOT EXISTS idx_territorios_uf_codigo ON territorios (ibge_uf_codigo);

-- 4. TABELA: CARGOS (Matriz Oficial RBAC — Marco 0.2 / Seção 3 do Plano)
CREATE TABLE IF NOT EXISTS cargos (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(50) UNIQUE NOT NULL,
    nome VARCHAR(100) NOT NULL,
    descricao TEXT,
    escopo_permitido tipo_escopo, -- NULL significa aplicável a qualquer escopo
    nivel_hierarquico INT NOT NULL,
    
    -- Matriz de Privilégios Granulares
    pode_gerenciar_infra BOOLEAN NOT NULL DEFAULT FALSE,
    pode_alterar_cargos BOOLEAN NOT NULL DEFAULT FALSE,
    pode_ativar_filiados BOOLEAN NOT NULL DEFAULT FALSE,
    pode_revogar_lote BOOLEAN NOT NULL DEFAULT FALSE,
    pode_publicar_nacional BOOLEAN NOT NULL DEFAULT FALSE,
    pode_publicar_estadual BOOLEAN NOT NULL DEFAULT FALSE,
    pode_publicar_municipal BOOLEAN NOT NULL DEFAULT FALSE,
    pode_solicitar_escalonamento BOOLEAN NOT NULL DEFAULT FALSE,
    pode_aprovar_escalonamento BOOLEAN NOT NULL DEFAULT FALSE,
    pode_votar_hype BOOLEAN NOT NULL DEFAULT FALSE,
    pode_leitura BOOLEAN NOT NULL DEFAULT TRUE,
    
    -- Limite operacional de governança
    limite_ativacao_semanal INT DEFAULT NULL,
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. TABELA: USUÁRIOS
-- ADR D4 & ADR D8: Vínculo territorial rígido, envelope 2FA AES-256-GCM e Rastreabilidade
CREATE TABLE IF NOT EXISTS usuarios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    nome VARCHAR(150) NOT NULL,
    email CITEXT UNIQUE NOT NULL,
    senha_hash VARCHAR(255) NOT NULL,
    cargo_id INT NOT NULL REFERENCES cargos(id) ON DELETE RESTRICT,
    
    -- Territorialidade estrita validada por restrição de integridade
    escopo tipo_escopo NOT NULL,
    ibge_codigo VARCHAR(7) REFERENCES territorios(codigo_ibge) ON DELETE RESTRICT,
    
    filiado_verificado BOOLEAN NOT NULL DEFAULT FALSE,
    status status_usuario NOT NULL DEFAULT 'PENDENTE',
    
    ativado_por UUID REFERENCES usuarios(id) ON DELETE RESTRICT,
    ativado_em TIMESTAMPTZ,
    
    -- ADR D8: Criptografia 2FA com envelope AES-256-GCM
    totp_secret_cifrado BYTEA,
    totp_habilitado BOOLEAN NOT NULL DEFAULT FALSE,
    recovery_codes_hashes TEXT[] DEFAULT '{}',
    
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    atualizado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- RESTRIÇÃO ADR D4: Usuários NACIONAIS não possuem IBGE. Usuários ESTADUAL/MUNICIPAL exigem código válido.
    CONSTRAINT chk_usuario_territorio_consistente CHECK (
        (escopo = 'NACIONAL' AND ibge_codigo IS NULL) OR
        (escopo IN ('ESTADUAL', 'MUNICIPAL') AND ibge_codigo IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_usuarios_cargo ON usuarios (cargo_id);
CREATE INDEX IF NOT EXISTS idx_usuarios_territorio ON usuarios (ibge_codigo);
CREATE INDEX IF NOT EXISTS idx_usuarios_ativado_por ON usuarios (ativado_por);
CREATE INDEX IF NOT EXISTS idx_usuarios_status ON usuarios (status);
CREATE INDEX IF NOT EXISTS idx_usuarios_filiado_ativo ON usuarios (filiado_verificado, status);

-- 6. TABELA: SESSÕES (ADR D3 — Cookies HTTP-Only + Rastreio de Invalidação)
CREATE TABLE IF NOT EXISTS sessoes (
    token_hash VARCHAR(64) PRIMARY KEY, -- Hash SHA-256 do token do cookie HTTP-Only
    usuario_id UUID NOT NULL REFERENCES usuarios(id) ON DELETE CASCADE,
    ip_origem INET NOT NULL,
    user_agent TEXT NOT NULL,
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expira_em TIMESTAMPTZ NOT NULL,
    ultimo_acesso TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sessoes_usuario ON sessoes (usuario_id);
CREATE INDEX IF NOT EXISTS idx_sessoes_expira ON sessoes (expira_em);

-- 7. TABELA: NOTÍCIAS (Editorial Federado, Sanitização Ammonia e Índices de Feed)
CREATE TABLE IF NOT EXISTS noticias (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    titulo VARCHAR(255) NOT NULL,
    slug VARCHAR(280) UNIQUE NOT NULL,
    subtitulo VARCHAR(500),
    autor_id UUID NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    
    -- Escopo da Matéria
    escopo tipo_escopo NOT NULL,
    ibge_codigo VARCHAR(7) REFERENCES territorios(codigo_ibge) ON DELETE RESTRICT,
    
    conteudo_original TEXT NOT NULL,
    conteudo_sanitizado TEXT NOT NULL, -- ADR D8: Processado exclusivamente via Crate Ammonia
    imagem_destaque_url VARCHAR(1024),
    
    status status_noticia NOT NULL DEFAULT 'RASCUNHO',
    
    -- Métricas de Engajamento Hype
    votos_positivos INT NOT NULL DEFAULT 0 CHECK (votos_positivos >= 0),
    votos_negativos INT NOT NULL DEFAULT 0 CHECK (votos_negativos >= 0),
    hype_score NUMERIC(12, 6) NOT NULL DEFAULT 0.0,
    
    publicado_em TIMESTAMPTZ,
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    atualizado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT chk_noticia_territorio CHECK (
        (escopo = 'NACIONAL' AND ibge_codigo IS NULL) OR
        (escopo IN ('ESTADUAL', 'MUNICIPAL') AND ibge_codigo IS NOT NULL)
    )
);

-- ADR D6: Índices comutáveis para Freeze Eleitoral
CREATE INDEX IF NOT EXISTS idx_noticias_feed_hype ON noticias (escopo, ibge_codigo, hype_score DESC, publicado_em DESC) 
    WHERE status = 'PUBLICADO';

CREATE INDEX IF NOT EXISTS idx_noticias_feed_cronologico ON noticias (escopo, ibge_codigo, publicado_em DESC) 
    WHERE status = 'PUBLICADO';

CREATE INDEX IF NOT EXISTS idx_noticias_autor ON noticias (autor_id);
CREATE INDEX IF NOT EXISTS idx_noticias_publicado_em ON noticias (publicado_em DESC);

-- 8. TABELA: SOLICITAÇÕES DE ESCALONAMENTO EDITORIAL
CREATE TABLE IF NOT EXISTS solicitacoes_escalonamento (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    noticia_id UUID NOT NULL REFERENCES noticias(id) ON DELETE CASCADE,
    solicitante_id UUID NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    escopo_origem tipo_escopo NOT NULL,
    escopo_destino tipo_escopo NOT NULL,
    status status_escalonamento NOT NULL DEFAULT 'PENDENTE',
    avaliado_por UUID REFERENCES usuarios(id) ON DELETE RESTRICT,
    justificativa TEXT NOT NULL,
    parecer_revisao TEXT,
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    avaliado_em TIMESTAMPTZ,
    CONSTRAINT chk_escalonamento_direcao CHECK (
        (escopo_origem = 'MUNICIPAL' AND escopo_destino = 'ESTADUAL') OR
        (escopo_origem = 'ESTADUAL' AND escopo_destino = 'NACIONAL')
    )
);

CREATE INDEX IF NOT EXISTS idx_escalonamento_status ON solicitacoes_escalonamento (status);
CREATE INDEX IF NOT EXISTS idx_escalonamento_noticia ON solicitacoes_escalonamento (noticia_id);

-- 9. TABELA: VOTOS_NOTICIAS (ADR D1 & ADR D2 — Ledger Imutável Auditável)
CREATE TABLE IF NOT EXISTS votos_noticias (
    id BIGSERIAL PRIMARY KEY,
    noticia_id UUID NOT NULL REFERENCES noticias(id) ON DELETE RESTRICT,
    usuario_id UUID NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    tipo_voto SMALLINT NOT NULL CHECK (tipo_voto IN (-1, 1)),
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Regra de Ouro: 1 Filiado = 1 Voto por notícia
    CONSTRAINT uq_votos_noticia_usuario UNIQUE (noticia_id, usuario_id)
);

CREATE INDEX IF NOT EXISTS idx_votos_noticia ON votos_noticias (noticia_id);
CREATE INDEX IF NOT EXISTS idx_votos_usuario_rate ON votos_noticias (usuario_id, criado_em DESC);

-- 10. TABELA: CONFIGURAÇÕES (ADR D6 — Feature Flag Modo Eleitoral & Parâmetros Globais)
CREATE TABLE IF NOT EXISTS configuracoes (
    chave VARCHAR(100) PRIMARY KEY,
    valor JSONB NOT NULL,
    descricao TEXT,
    modificado_por UUID REFERENCES usuarios(id) ON DELETE RESTRICT,
    atualizado_em TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 11. TABELA: AUDITORIA_LOGS (Governança, TSE e Rastreabilidade Integral)
CREATE TABLE IF NOT EXISTS auditoria_logs (
    id BIGSERIAL PRIMARY KEY,
    ator_id UUID REFERENCES usuarios(id) ON DELETE SET NULL,
    acao VARCHAR(100) NOT NULL,
    entidade VARCHAR(50) NOT NULL,
    entidade_id VARCHAR(100),
    dados_anteriores JSONB,
    dados_novos JSONB,
    ip_origem INET NOT NULL,
    user_agent TEXT,
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auditoria_ator ON auditoria_logs (ator_id);
CREATE INDEX IF NOT EXISTS idx_auditoria_acao ON auditoria_logs (acao);
CREATE INDEX IF NOT EXISTS idx_auditoria_criado_em ON auditoria_logs (criado_em DESC);

-- =============================================================================
-- TRIGGERS DE GOVERNANÇA E IMUTABILIDADE
-- =============================================================================

-- Atualização automática de timestamp atualizado_em
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.atualizado_em = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_timestamp_usuarios
BEFORE UPDATE ON usuarios
FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

CREATE TRIGGER set_timestamp_noticias
BEFORE UPDATE ON noticias
FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

CREATE TRIGGER set_timestamp_configuracoes
BEFORE UPDATE ON configuracoes
FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- ADR D2: Garantia física de Imutabilidade do Ledger de Votos
CREATE OR REPLACE FUNCTION trigger_guard_votos_imutabilidade()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'VIOLAÇÃO DE COMPLIANCE ELEITORAL: Registros de votos na tabela votos_noticias são estritamente imutáveis e não permitem UPDATE ou DELETE.';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER guard_votos_noticias_no_update_delete
BEFORE UPDATE OR DELETE ON votos_noticias
FOR EACH ROW EXECUTE FUNCTION trigger_guard_votos_imutabilidade();

COMMIT;
