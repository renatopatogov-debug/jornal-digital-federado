-- =============================================================================
-- SEED: 20261102000002_seeds_fundacionais.sql
-- Homologação dos 9 Cargos RBAC, Configurações de Compliance e Territórios Iniciais
-- =============================================================================

BEGIN;

-- 1. CARGA DOS 9 CARGOS OFICIAIS (Marco 0.2 / Seção 3 do Plano)
INSERT INTO cargos (
    id, slug, nome, descricao, escopo_permitido, nivel_hierarquico,
    pode_gerenciar_infra, pode_alterar_cargos, pode_ativar_filiados, pode_revogar_lote,
    pode_publicar_nacional, pode_publicar_estadual, pode_publicar_municipal,
    pode_solicitar_escalonamento, pode_aprovar_escalonamento, pode_votar_hype, pode_leitura,
    limite_ativacao_semanal
) VALUES 
(1, 'superadmin', 'SuperAdmin (Infra/Segurança)', 'Custódia técnica, gestão de infraestrutura, logs e auditoria.', NULL, 1, 
 TRUE, TRUE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, TRUE, TRUE, NULL),

(2, 'lider_nacional', 'Líder Nacional', 'Executiva Nacional: gestão política, revogação em lote geral e aprovação macro.', 'NACIONAL', 2, 
 FALSE, TRUE, TRUE, TRUE, TRUE, TRUE, TRUE, FALSE, TRUE, TRUE, TRUE, NULL),

(3, 'redator_nacional', 'Redator Nacional', 'Corpo editorial nacional: redação, curadoria e aprovação de escalonamento.', 'NACIONAL', 3, 
 FALSE, FALSE, FALSE, FALSE, TRUE, FALSE, FALSE, FALSE, TRUE, TRUE, TRUE, NULL),

(4, 'lider_estadual', 'Líder Estadual', 'Diretório Regional (UF): ativação e contenção territorial na unidade federativa.', 'ESTADUAL', 4, 
 FALSE, FALSE, TRUE, TRUE, FALSE, TRUE, TRUE, FALSE, TRUE, TRUE, TRUE, NULL),

(5, 'redator_estadual', 'Redator Estadual', 'Produção editorial na UF e solicitação de escalonamento para o Feed Nacional.', 'ESTADUAL', 5, 
 FALSE, FALSE, FALSE, FALSE, FALSE, TRUE, TRUE, TRUE, FALSE, TRUE, TRUE, NULL),

(6, 'lider_municipal', 'Líder Municipal', 'Diretório Municipal: ativação de filiados com cota estrita de 50/semana.', 'MUNICIPAL', 6, 
 FALSE, FALSE, TRUE, FALSE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, TRUE, 50),

(7, 'redator_municipal', 'Redator Municipal', 'Produção editorial do município e solicitação de escalonamento para a UF.', 'MUNICIPAL', 7, 
 FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, TRUE, TRUE, FALSE, TRUE, TRUE, NULL),

(8, 'filiado', 'Filiado', 'Membro partidário verificado com direito de voto auditável no ranking Hype.', NULL, 8, 
 FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, TRUE, TRUE, NULL),

(9, 'leitor', 'Leitor', 'Público geral com acesso irrestrito de leitura não autenticado.', NULL, 9, 
 FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, TRUE, NULL)
ON CONFLICT (slug) DO UPDATE SET 
    nome = EXCLUDED.nome,
    limite_ativacao_semanal = EXCLUDED.limite_ativacao_semanal;

SELECT setval('cargos_id_seq', (SELECT MAX(id) FROM cargos));

-- 2. CARGA DE CONFIGURAÇÕES FUNDACIONAIS (ADR D6 — Modo Eleitoral & Parâmetros Hype)
INSERT INTO configuracoes (chave, valor, descricao) VALUES
('modo_eleitoral', '{
    "ativo": false,
    "banner_obrigatorio": true,
    "banner_texto": "Exibição estritamente cronológica em observância à legislação eleitoral (Lei nº 9.504/97).",
    "freeze_hype": false,
    "motivo": "Operação Ordinária (Fora do Período Eleitoral Defeso)"
}'::jsonb, 'Contrato Operacional de Contingência do TSE / Lei 9.504/97'),

('hype_parameters', '{
    "g_nacional": 1.8,
    "g_regional": 1.4,
    "offset_horas": 2.0,
    "min_votos_qualificados": 10
}'::jsonb, 'Parâmetros públicos da equação de decaimento gravitacional Hype'),

('rate_limits', '{
    "votos_por_minuto": 10,
    "max_ativacoes_semana_lider_municipal": 50,
    "max_noticias_dia_redator_municipal": 5
}'::jsonb, 'Tetos operacionais de contenção anti-abuso')
ON CONFLICT (chave) DO NOTHING;

-- 3. CARGA DE TERRITÓRIOS (27 UFs Federadas)
INSERT INTO territorios (codigo_ibge, nome, tipo, uf, ibge_uf_codigo, populacao_estimada) VALUES
('11', 'Rondônia', 'ESTADUAL', 'RO', '11', 1581196),
('12', 'Acre', 'ESTADUAL', 'AC', '12', 830018),
('13', 'Amazonas', 'ESTADUAL', 'AM', '13', 3941613),
('14', 'Roraima', 'ESTADUAL', 'RR', '14', 636707),
('15', 'Pará', 'ESTADUAL', 'PA', '15', 8120131),
('16', 'Amapá', 'ESTADUAL', 'AP', '16', 733759),
('17', 'Tocantins', 'ESTADUAL', 'TO', '17', 1511460),
('21', 'Maranhão', 'ESTADUAL', 'MA', '21', 6776699),
('22', 'Piauí', 'ESTADUAL', 'PI', '22', 3271199),
('23', 'Ceará', 'ESTADUAL', 'CE', '23', 8794957),
('24', 'Rio Grande do Norte', 'ESTADUAL', 'RN', '24', 3303953),
('25', 'Paraíba', 'ESTADUAL', 'PB', '25', 3974687),
('26', 'Pernambuco', 'ESTADUAL', 'PE', '26', 9058931),
('27', 'Alagoas', 'ESTADUAL', 'AL', '27', 3127683),
('28', 'Sergipe', 'ESTADUAL', 'SE', '28', 2210004),
('29', 'Bahia', 'ESTADUAL', 'BA', '29', 14141626),
('31', 'Minas Gerais', 'ESTADUAL', 'MG', '31', 20539989),
('32', 'Espírito Santo', 'ESTADUAL', 'ES', '32', 3833712),
('33', 'Rio de Janeiro', 'ESTADUAL', 'RJ', '33', 16055174),
('35', 'São Paulo', 'ESTADUAL', 'SP', '35', 44411238),
('41', 'Paraná', 'ESTADUAL', 'PR', '41', 11444380),
('42', 'Santa Catarina', 'ESTADUAL', 'SC', '42', 7610361),
('43', 'Rio Grande do Sul', 'ESTADUAL', 'RS', '43', 10882965),
('50', 'Mato Grosso do Sul', 'ESTADUAL', 'MS', '50', 2757013),
('51', 'Mato Grosso', 'ESTADUAL', 'MT', '51', 3658649),
('52', 'Goiás', 'ESTADUAL', 'GO', '52', 7056495),
('53', 'Distrito Federal', 'ESTADUAL', 'DF', '53', 2817381)
ON CONFLICT (codigo_ibge) DO NOTHING;

-- CARGA DE MUNICÍPIOS-PILOTO INICIAIS
INSERT INTO territorios (codigo_ibge, nome, tipo, uf, ibge_uf_codigo, populacao_estimada) VALUES
('3550308', 'São Paulo', 'MUNICIPAL', 'SP', '35', 11451999),
('3304557', 'Rio de Janeiro', 'MUNICIPAL', 'RJ', '33', 6211223),
('3106200', 'Belo Horizonte', 'MUNICIPAL', 'MG', '31', 2315560),
('4106900', 'Curitiba', 'MUNICIPAL', 'PR', '41', 1773718),
('5300108', 'Brasília', 'MUNICIPAL', 'DF', '53', 2817381)
ON CONFLICT (codigo_ibge) DO NOTHING;

-- 4. CARGA DE USUÁRIOS INICIAIS DE CUSTÓDIA (Marco 0.4 de Segurança)
-- As senhas são geradas via Argon2id para os custodiantes homologados
INSERT INTO usuarios (
    id, nome, email, senha_hash, cargo_id, escopo, ibge_codigo, filiado_verificado, status
) VALUES
('00000000-0000-0000-0000-000000000001'::uuid, 'Custodiante SuperAdmin (TI)', 'superadmin@jornalfederado.org.br', 
 '$argon2id$v=19$m=65536,t=3,p=4$JornalFedSeedSalt01$7tYyvQn4F5gW2m1Hk8L0XvR6zO4q9P3w8Y7x6c5v4b3', 
 1, 'NACIONAL', NULL, TRUE, 'ATIVO'),

('00000000-0000-0000-0000-000000000002'::uuid, 'Presidência Executiva Nacional', 'lider.nacional@jornalfederado.org.br',
 '$argon2id$v=19$m=65536,t=3,p=4$JornalFedSeedSalt02$9uZaXk8N5vQ2m1Hk8L0XvR6zO4q9P3w8Y7x6c5v4b3', 
 2, 'NACIONAL', NULL, TRUE, 'ATIVO')
ON CONFLICT (email) DO NOTHING;

COMMIT;
