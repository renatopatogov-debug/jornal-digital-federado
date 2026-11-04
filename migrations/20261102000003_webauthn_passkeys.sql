-- =============================================================================
-- MIGRATION: 20261102000003_webauthn_passkeys.sql
-- PROJETO: Jornal Digital Federado (Plano Diretor v2.1)
-- COMPATIBILIDADE: PostgreSQL 16+
-- DESCRIÇÃO: Suporte nativo a Passkeys / WebAuthn (FIDO2 / Biometria)
-- =============================================================================

BEGIN;

CREATE TABLE IF NOT EXISTS webauthn_passkeys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    usuario_id UUID NOT NULL REFERENCES usuarios(id) ON DELETE CASCADE,
    credencial_id BYTEA UNIQUE NOT NULL,
    chave_publica BYTEA NOT NULL,
    contador_assinatura BIGINT NOT NULL DEFAULT 0,
    aaguid UUID,
    nome_dispositivo VARCHAR(100) NOT NULL DEFAULT 'Dispositivo Biométrico',
    criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ultimo_uso TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_passkeys_usuario ON webauthn_passkeys (usuario_id);
CREATE INDEX IF NOT EXISTS idx_passkeys_credencial ON webauthn_passkeys (credencial_id);

-- Flag opcional para usuários que possuem Passkeys registradas
ALTER TABLE usuarios ADD COLUMN IF NOT EXISTS passkeys_habilitadas BOOLEAN NOT NULL DEFAULT FALSE;

COMMIT;
