#!/usr/bin/env python3
"""
Utilitário de Controle e Simulação do Interruptor "Modo Eleitoral" (ADR D6)
Plano Diretor v2.1 — Fase 6 & Go-Live

Permite à Diretoria Jurídica ou à Presidência Nacional comutar instantaneamente
a flag global de compliance com a Lei nº 9.504/97.
"""

import os
import sys
import json
import urllib.error
import urllib.request

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def alternar_modo_eleitoral(ativo: bool, motivo: str = "Ajuste operacional de conformidade"):
    status_str = "ATIVADO (Freeze do Hype / Lei 9.504/97)" if ativo else "DESATIVADO (Operacao Ordinaria)"
    print("=" * 65)
    print("  SIMULADOR DO INTERRUPTOR DE MODO ELEITORAL - JORNAL DIGITAL FEDERADO")
    print("=" * 65)
    print(f"[STATUS ALVO] Definindo Modo Eleitoral como: {status_str}")
    print(f"[JUSTIFICATIVA] {motivo}\n")

    api_url = os.environ.get("API_URL", "http://127.0.0.1:8080").rstrip("/")
    session_cookie = os.environ.get("SESSION_COOKIE", "").strip()

    if session_cookie:
        payload_bytes = json.dumps({"ativo": ativo, "motivo": motivo}).encode("utf-8")
        req = urllib.request.Request(
            f"{api_url}/api/v1/infra/modo-eleitoral",
            data=payload_bytes,
            method="POST",
            headers={
                "Content-Type": "application/json",
                "Cookie": session_cookie,
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                dados = json.loads(resp.read().decode("utf-8"))
                print(f"[API HTTP {resp.status}] {dados.get('mensagem', 'Atualizado na API.')}")
        except urllib.error.URLError as exc:
            print(f"[AVISO API] Nao foi possivel acionar {api_url}/api/v1/infra/modo-eleitoral: {exc}")

    motivo_json_sql = json.dumps(motivo, ensure_ascii=False).replace("'", "''")
    dados_auditoria_sql = json.dumps(
        {"ativo": ativo, "freeze_hype": ativo, "motivo": motivo},
        ensure_ascii=False,
    ).replace("'", "''")
    bool_sql = "true" if ativo else "false"

    # Comando SQL oficial parametrizado/escapado para PostgreSQL 16
    comando_sql = f"""
    UPDATE configuracoes
    SET valor = jsonb_set(
        jsonb_set(
            jsonb_set(valor, '{{ativo}}', '{bool_sql}'),
            '{{freeze_hype}}', '{bool_sql}'
        ),
        '{{motivo}}', '{motivo_json_sql}'::jsonb
    ),
    atualizado_em = NOW()
    WHERE chave = 'modo_eleitoral';

    INSERT INTO auditoria_logs (acao, entidade, entidade_id, dados_novos, ip_origem)
    VALUES (
        'MODO_ELEITORAL_ALTERADO',
        'configuracoes',
        'modo_eleitoral',
        '{dados_auditoria_sql}'::jsonb,
        '127.0.0.1'::inet
    );
    """

    print("[COMANDO SQL DE GOVERNANCA GERADO]:")
    print(comando_sql.strip())
    print("\n[EFEITOS SISTEMICOS IMEDIATOS]:")
    if ativo:
        print("  1. As consultas publicas forcam ordenacao cronologica pura (publicado_em DESC).")
        print("  2. Os placares numericos de Hype sao ocultados em todo o portal.")
        print("  3. O banner obrigatorio da Lei 9.504/97 e fixado no topo do site.")
        print("  4. O worker Tokio em Rust mantem o recalculo silencioso em segundo plano.")
    else:
        print("  1. O ranking algoritmico do Hype Score volta a ordenar as materias qualificadas.")
        print("  2. Os placares visuais de engajamento sao restaurados.")
        print("  3. O banner legal de periodo defeso e removido.")

    print("\n[OK] Simulacao concluida com sucesso.")
    print("=" * 65)

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1].lower() in ["true", "1", "ativar", "on"]:
        alternar_modo_eleitoral(True, "Início do Período Eleitoral Defeso (Lei nº 9.504/97)")
    else:
        alternar_modo_eleitoral(False, "Operação Ordinária Fora do Período Eleitoral")
