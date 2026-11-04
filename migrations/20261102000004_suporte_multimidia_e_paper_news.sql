-- =============================================================================
-- MIGRATION: 20261102000004_suporte_multimidia_e_paper_news.sql
-- PROJETO: Jornal Digital Federado (Plano Diretor v2.1)
-- DESCRIÇÃO: Suporte nativo a Vídeos, Áudios (Podcasts/Rádio) e Paper News (PDF)
-- =============================================================================

BEGIN;

-- 1. Tipo enumerado para formatos de matéria
DO $$ BEGIN
    CREATE TYPE tipo_formato_materia AS ENUM ('TEXTO', 'VIDEO', 'AUDIO', 'IMPRESSO_PDF');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 2. Expansão da tabela de notícias com campos multimídia
ALTER TABLE noticias 
    ADD COLUMN IF NOT EXISTS formato tipo_formato_materia NOT NULL DEFAULT 'TEXTO',
    ADD COLUMN IF NOT EXISTS video_url VARCHAR(1024),
    ADD COLUMN IF NOT EXISTS audio_url VARCHAR(1024),
    ADD COLUMN IF NOT EXISTS audio_duracao_segundos INT,
    ADD COLUMN IF NOT EXISTS pdf_edicao_url VARCHAR(1024),
    ADD COLUMN IF NOT EXISTS pdf_numero_paginas INT;

CREATE INDEX IF NOT EXISTS idx_noticias_formato ON noticias (formato);

COMMIT;
