# Jornal Digital Federado — Plataforma Editorial e Participativa

Projeto estruturado conforme as diretrizes do **Plano Diretor Executivo v2.1** e as salvaguardas de integridade eleitoral (**Lei nº 9.504/97** e **ADRs D1 a D8**).

---

## 🏛️ Visão Geral da Arquitetura

O **Jornal Digital Federado** é uma plataforma distribuída em 3 esferas político-editoriais:
- **Nacional:** Cobertura macro, diretrizes nacionais e aprovação de escalonamento.
- **Estadual:** 27 Unidades da Federação com feeds regionais independentes.
- **Municipal:** 5.570 municípios brasileiros com cobertura hiperlocal.

### Principais Salvaguardas Implementadas
1. **Leitura 100% Aberta & Voto Único Auditável (ADR D1):** O público lê sem cadastro. O voto no ranking Hype é restrito a filiados formalmente verificados (`1 filiado = 1 voto`).
2. **Imutabilidade do Ledger de Votos (ADR D2):** Trigger físico no banco bloqueando qualquer `UPDATE` ou `DELETE` na tabela `votos_noticias`.
3. **RBAC Territorial no PostgreSQL (ADR D4):** Validação via `CHECK constraint` no banco garantindo que usuários operem estritamente dentro da sua jurisdição IBGE.
4. **Interruptor "Modo Eleitoral" (ADR D6):** Feature flag global que comuta os feeds instantaneamente de algorítmico (`hype_score DESC`) para cronológico puro (`publicado_em DESC`) com banner legal obrigatório.
5. **Sanitização XSS Estrita (ADR D8):** Processamento do HTML via crate Rust `ammonia` antes de qualquer persistência.

---

## 📂 Estrutura de Diretórios

```
jornal-digital-federado/
├── .github/
│   └── workflows/
│       └── ci.yml                     # Pipeline CI/CD com checagem de DDL, lint e GeoJSON
├── backend/                           # Core API em Rust (Axum + SQLx)
│   ├── Cargo.toml                     # Dependências (Axum, SQLx, Tokio, Ammonia, Argon2)
│   └── src/
│       ├── main.rs                    # Ponto de entrada da API e servidor HTTP
│       ├── config.rs                  # Gestão de variáveis de ambiente
│       ├── error.rs                   # Tratamento tipado de erros HTTP
│       ├── middleware/
│       │   └── rbac.rs                # Validação de token de sessão e RBAC territorial
│       ├── models/
│       │   └── mod.rs                 # Estruturas de dados (Território, Usuário, Notícia)
│       └── routes/
│           ├── mod.rs                 # Registro unificado de rotas
│           ├── health.rs              # Endpoint de verificação de integridade
│           └── feed.rs                # Feed com chaveamento automático de Modo Eleitoral
├── migrations/                        # Scripts SQLx compatíveis com PostgreSQL 16
│   ├── 20261102000001_schema_inicial.sql    # DDL das 9 tabelas, constraints e triggers
│   └── 20261102000002_seeds_fundacionais.sql # Carga inicial de cargos, UFs e configs
├── scripts/
│   ├── pipeline_ibge_geojson.py       # Fatiamento da malha IBGE em 27 UFs (<250KB cada)
│   └── setup_dev.py                   # Utilitário Python para inicialização e testes locais
├── docker-compose.yml                 # PostgreSQL 16 Multi-AZ + Redis 7
├── .env.example                       # Exemplo de variáveis de ambiente
└── README.md                          # Este guia
```

---

## 🚀 Como Iniciar o Projeto Localmente

### Pré-requisitos
- **PostgreSQL 16** (ou Docker Desktop rodando o `docker-compose.yml`)
- **Python 3.11+** (para o pipeline geográfico e scripts auxiliares)
- **Rust Toolchain (1.75+)** (quando for compilar o backend Axum)

### Passo 1: Subir a Infraestrutura (Banco e Cache)
Se estiver utilizando Docker:
```bash
docker compose up -d
```
Ou configure uma instância local do PostgreSQL 16 e crie o banco:
```sql
CREATE DATABASE jornal_federado;
```

### Passo 2: Configurar o Arquivo `.env`
Copie o modelo de ambiente:
```bash
cp .env.example .env
```

### Passo 3: Executar as Migrações SQL
Com o utilitário `psql` ou via `sqlx-cli`:
```bash
psql -h localhost -U postgres -d jornal_federado -f migrations/20261102000001_schema_inicial.sql
psql -h localhost -U postgres -d jornal_federado -f migrations/20261102000002_seeds_fundacionais.sql
```
*Ou utilize o script de automação:*
```bash
python scripts/setup_dev.py --run-migrations
```

### Passo 4: Processar a Malha IBGE (GeoJSON por UF)
Para gerar os 27 arquivos de UF otimizados para o mapa:
```bash
python scripts/pipeline_ibge_geojson.py
```

### Passo 5: Iniciar o Backend em Rust (Axum)
```bash
cd backend
cargo run
```
A API responderá em `http://127.0.0.1:8080`:
- **Health Check:** `GET /health`
- **Feed Nacional:** `GET /api/v1/feed`
- **Feed por UF:** `GET /api/v1/feed?uf=SP`
- **Feed Municipal:** `GET /api/v1/feed?ibge=3550308`
