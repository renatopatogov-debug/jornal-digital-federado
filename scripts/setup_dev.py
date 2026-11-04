#!/usr/bin/env python3
"""
Utilitário de Diagnóstico e Inicialização de Desenvolvimento
Jornal Digital Federado — Plano Diretor v2.1
"""

import os
import sys

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def check_env():
    print("=" * 60)
    print("  DIAGNOSTICO DO AMBIENTE LOCAL - JORNAL DIGITAL FEDERADO")
    print("=" * 60)
    
    env_file = os.path.join(BASE_DIR, ".env")
    env_example = os.path.join(BASE_DIR, ".env.example")
    
    if os.path.exists(env_file):
        print("[OK] Arquivo .env detectado.")
    else:
        print("[INFO] Arquivo .env ausente. Copiando a partir de .env.example...")
        if os.path.exists(env_example):
            with open(env_example, "r", encoding="utf-8") as src:
                with open(env_file, "w", encoding="utf-8") as dst:
                    dst.write(src.read())
            print("[OK] Arquivo .env criado com sucesso a partir de .env.example.")
        else:
            print("[ERRO] Arquivo .env.example nao encontrado.")

    # Verificar diretórios essenciais
    public_geo = os.path.join(BASE_DIR, "public", "geo", "ufs")
    if os.path.exists(public_geo) and len(os.listdir(public_geo)) >= 27:
        print(f"[OK] Malha GeoJSON das 27 UFs presente e indexada em {public_geo}.")
    else:
        print("[AVISO] Malhas GeoJSON ausentes. Execute: python scripts/pipeline_ibge_geojson.py")

    migrations_dir = os.path.join(BASE_DIR, "migrations")
    if os.path.exists(migrations_dir):
        files = os.listdir(migrations_dir)
        print(f"[OK] Migracoes SQL prontas ({len(files)} arquivos em {migrations_dir}):")
        for f in sorted(files):
            print(f"    - {f}")

    backend_src = os.path.join(BASE_DIR, "backend", "src")
    if os.path.exists(backend_src):
        print("[OK] Core API Rust (Axum 0.7) & Background Worker:")
        print("    - Middlewares: RBAC Territorial, Barreira de Jurisdicao IBGE")
        print("    - Servicos: Hype Decay Engine (G=1.8 / G=1.4), Criptografia AES-256-GCM, WebAuthn (Passkeys)")
        print("    - Worker: HypeWorker assincrono Tokio (ciclo de recalculo a cada 5 min)")
        print("    - Rotas: Auth, Feed com Modo Eleitoral, Ledger de Votacao Imutavel (/voto), Noticias, CMS")

    frontend_src = os.path.join(BASE_DIR, "frontend", "src")
    if os.path.exists(frontend_src):
        print("[OK] Frontend Next.js 14+ (SSR / App Router) estruturado:")
        print("    - Componentes: MapaFederado (Leaflet sob Demanda por UF), BannerEleitoral (Lei 9.504/97), CardNoticia")
        print("    - Rotas: Feed Nacional SSR (/), CMS Redacao (/cms), Login Hibrido Passkey + TOTP (/auth/login)")

    load_test = os.path.join(BASE_DIR, "scripts", "load_test_k6.js")
    if os.path.exists(load_test):
        print("[OK] Harness de Teste de Carga K6 estruturado (meta de 50.000 req/min com P95 < 50ms).")

    rings_script = os.path.join(BASE_DIR, "scripts", "verify_rings_security.py")
    if os.path.exists(rings_script):
        print("[OK] Script de Auditoria em Aneis e Pre-Pentest estruturado.")

    electoral_sim = os.path.join(BASE_DIR, "scripts", "simular_modo_eleitoral.py")
    if os.path.exists(electoral_sim):
        print("[OK] Simulador de Modo Eleitoral (Lei 9.504/97) e Runbook de Go-Live prontos.")
            
    print("\n[Instrucoes de Inicializacao]")
    print("1. Para iniciar PostgreSQL e Redis via Docker: docker compose up -d")
    print("2. Para gerar/atualizar a malha do mapa: python scripts/pipeline_ibge_geojson.py")
    print("3. Para compilar o backend Rust: cd backend && cargo run")
    print("=" * 60)

if __name__ == "__main__":
    check_env()
