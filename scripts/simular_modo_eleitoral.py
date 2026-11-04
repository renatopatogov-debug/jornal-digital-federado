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
import urllib.request

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def alternar_modo_eleitoral(ativo: bool, motivo: str = "Ajuste operacional de conformidade"):
    status_str = "ATIVADO (Freeze do Hype / Lei 9.504/97)" if ativo else "DESATIVADO (Operacao Ordinaria)"
    print("=" * 65)
    print("  SIMULADOR DO INTERRUPTOR DE MODO ELEITORAL - JORNAL DIGITAL FEDERADO")
    print("=" * 65)
    print(f"[STATUS ALVO] Definindo Modo Eleitoral como: {status_str}")
    print(f"[JUSTIFICATIVA] {motivo}\n")

    # Comando SQL oficial a ser executado no PostgreSQL 16
    comando_sql = f"""
    UPDATE configuracoes 
    SET valor = jsonb_set(
        jsonb_set(valor, '{{ativo}}', '{str(ativo).lower()}'),
        '{{motivo}}', '"{motivo}"'
    ),
    atualizado_em = NOW()
    WHERE chave = 'modo_eleitoral';

    INSERT INTO auditoria_logs (acao, entidade, dados_novos, ip_origem)
    VALUES (
        'MODO_ELEITORAL_ALTERADO',
        'configuracoes',
        '{{"ativo": {str(ativo).lower()}, "motivo": "{motivo}"}}'::jsonb,
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
