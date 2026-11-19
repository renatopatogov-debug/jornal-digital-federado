"use client";

import React, { useState } from "react";
import { ThumbsUp, ThumbsDown, CheckCircle2, AlertCircle, LogIn } from "lucide-react";

interface BotaoVotarProps {
  noticiaId: string;
  votosPositivosIniciais: number;
  votosNegativosIniciais: number;
  modoEleitoral: boolean;
}

export const BotaoVotar: React.FC<BotaoVotarProps> = ({
  noticiaId,
  votosPositivosIniciais,
  votosNegativosIniciais,
  modoEleitoral,
}) => {
  const [positivos, setPositivos] = useState(votosPositivosIniciais);
  const [negativos, setNegativos] = useState(votosNegativosIniciais);
  const [votou, setVotou] = useState<number | null>(null);
  const [carregando, setCarregando] = useState(false);
  const [mensagem, setMensagem] = useState<string | null>(null);
  const [erroVoto, setErroVoto] = useState(false);
  const [requerLogin, setRequerLogin] = useState(false);

  const submeterVoto = async (tipo: number) => {
    if (votou !== null) return;
    setMensagem(null);
    setErroVoto(false);
    setRequerLogin(false);

    setCarregando(true);
    try {
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
      const res = await fetch(`${apiUrl}/api/v1/noticias/${noticiaId}/voto`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          tipo_voto: tipo,
        }),
      });

      const data: {
        mensagem?: string;
        votos_positivos?: number;
        votos_negativos?: number;
      } | null = await res.json().catch(() => null);
      if (res.ok) {
        setVotou(tipo);
        if (typeof data?.votos_positivos === "number") {
          setPositivos(data.votos_positivos);
        } else if (tipo === 1) {
          setPositivos((p) => p + 1);
        }
        if (typeof data?.votos_negativos === "number") {
          setNegativos(data.votos_negativos);
        } else if (tipo === -1) {
          setNegativos((n) => n + 1);
        }
        setMensagem(
          data?.mensagem ||
            "Seu voto foi persistido no ledger auditável (1 filiado = 1 voto)."
        );
      } else if (res.status === 401 || res.status === 403) {
        setRequerLogin(true);
        setErroVoto(true);
        setMensagem(data?.mensagem || "Apenas filiados verificados podem votar no Hype.");
      } else {
        setErroVoto(true);
        setMensagem(data?.mensagem || "Não foi possível computar o voto.");
      }
    } catch {
      setErroVoto(true);
      setMensagem("Não foi possível conectar ao ledger de votos. Tente novamente em instantes.");
    } finally {
      setCarregando(false);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        {/* Botão Upvote (Hype) */}
        <button
          onClick={() => submeterVoto(1)}
          disabled={votou !== null || carregando}
          className={`flex items-center gap-2 px-4 py-2 rounded-xl font-semibold text-sm transition-all border ${
            votou === 1
              ? "bg-emerald-600 text-white border-emerald-600 shadow-sm"
              : "bg-white text-slate-700 hover:border-emerald-500 hover:text-emerald-600 border-slate-200"
          } disabled:opacity-75`}
        >
          <ThumbsUp className={`w-4 h-4 ${votou === 1 ? "fill-white" : ""}`} />
          <span>Apoiar Pauta</span>
          {!modoEleitoral && (
            <span className="bg-slate-100 text-slate-600 px-2 py-0.5 rounded-full text-xs font-bold ml-1">
              {positivos}
            </span>
          )}
        </button>

        {/* Botão Downvote (Rejeição) */}
        <button
          onClick={() => submeterVoto(-1)}
          disabled={votou !== null || carregando}
          className={`flex items-center gap-2 px-4 py-2 rounded-xl font-semibold text-sm transition-all border ${
            votou === -1
              ? "bg-red-600 text-white border-red-600 shadow-sm"
              : "bg-white text-slate-700 hover:border-red-500 hover:text-red-600 border-slate-200"
          } disabled:opacity-75`}
        >
          <ThumbsDown className={`w-4 h-4 ${votou === -1 ? "fill-white" : ""}`} />
          <span>Discordar</span>
          {!modoEleitoral && (
            <span className="bg-slate-100 text-slate-600 px-2 py-0.5 rounded-full text-xs font-bold ml-1">
              {negativos}
            </span>
          )}
        </button>
      </div>

      {mensagem && (
        <div
          className={`text-xs p-3 rounded-lg flex items-center justify-between gap-2 ${
            requerLogin || erroVoto
              ? "bg-amber-50 text-amber-800 border border-amber-200"
              : "bg-emerald-50 text-emerald-800 border border-emerald-200"
          }`}
        >
          <div className="flex items-center gap-2">
            {requerLogin || erroVoto ? (
              <AlertCircle className="w-4 h-4 text-amber-600 flex-shrink-0" />
            ) : (
              <CheckCircle2 className="w-4 h-4 text-emerald-600 flex-shrink-0" />
            )}
            <span>{mensagem}</span>
          </div>

          {requerLogin && (
            <a
              href="/auth/filiado"
              className="flex items-center gap-1 font-bold text-amber-900 underline text-[11px]"
            >
              <LogIn className="w-3.5 h-3.5" />
              Entrar como Filiado
            </a>
          )}
        </div>
      )}
    </div>
  );
};
