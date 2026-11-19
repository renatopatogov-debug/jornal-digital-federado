"use client";

import React, { useEffect, useState } from "react";
import { LogOut, UserCheck } from "lucide-react";
import { buscarSessaoAtual, encerrarSessao, SessaoUsuario } from "@/lib/auth";

export const BotaoSessaoHeader: React.FC = () => {
  const [sessao, setSessao] = useState<SessaoUsuario | null>(null);
  const [saindo, setSaindo] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    buscarSessaoAtual(controller.signal).then((data) => {
      if (!controller.signal.aborted) {
        setSessao(data);
      }
    });
    return () => controller.abort();
  }, []);

  const handleLogout = async () => {
    setSaindo(true);
    await encerrarSessao();
    setSessao(null);
    window.location.href = "/";
  };

  if (!sessao) {
    return (
      <a
        href="/auth/login"
        className="bg-destaque hover:bg-blue-700 text-white px-3.5 py-1.5 rounded-lg text-xs font-semibold shadow-sm transition-colors"
      >
        Entrar
      </a>
    );
  }

  return (
    <div className="flex items-center gap-2">
      <span
        className="hidden md:inline-flex items-center gap-1.5 text-xs text-slate-200 bg-slate-800 border border-slate-700 px-2.5 py-1 rounded-lg"
        title={`${sessao.cargo_nome} (${sessao.escopo})`}
      >
        <UserCheck className="w-3.5 h-3.5 text-emerald-400" />
        <span className="max-w-[140px] truncate">{sessao.nome}</span>
      </span>
      <button
        type="button"
        onClick={handleLogout}
        disabled={saindo}
        className="inline-flex items-center gap-1.5 bg-slate-800 hover:bg-red-700 text-slate-200 hover:text-white border border-slate-700 px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors disabled:opacity-60"
      >
        <LogOut className="w-3.5 h-3.5" />
        {saindo ? "Saindo..." : "Sair"}
      </button>
    </div>
  );
};
