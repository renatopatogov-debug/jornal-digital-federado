"use client";

import React, { useState } from "react";
import Link from "next/link";
import { Vote, Mail, Hash, ArrowRight, CheckCircle2, AlertCircle } from "lucide-react";

export default function LoginFiliadoPage() {
  const [identificador, setIdentificador] = useState("");
  const [email, setEmail] = useState("");
  const [carregando, setCarregando] = useState(false);
  const [mensagemSucesso, setMensagemSucesso] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  const handleValidarFiliado = async (e: React.FormEvent) => {
    e.preventDefault();
    setCarregando(true);
    setErro(null);
    setMensagemSucesso(null);

    try {
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
      const res = await fetch(`${apiUrl}/api/v1/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ email, senha: identificador }),
      });

      const data: {
        sucesso?: boolean;
        requer_2fa?: boolean;
        usuario_id?: string;
        mensagem?: string;
      } | null = await res.json().catch(() => null);

      if (!res.ok || !data?.sucesso) {
        setErro(
          data?.mensagem ||
            "Credenciais inválidas ou filiado ainda não ativado pelo diretório."
        );
        return;
      }

      if (data.usuario_id) {
        localStorage.setItem("jf_usuario_id", data.usuario_id);
      }

      if (data.requer_2fa) {
        setErro(
          "Esta conta exige confirmação de 2º fator (2FA). Utilize o portal administrativo ou a página de login completo."
        );
        return;
      }

      setMensagemSucesso(
        data.mensagem ||
          "Credencial de votante ativada com sucesso! Redirecionando para o jornal..."
      );

      setTimeout(() => {
        window.location.href = "/";
      }, 1500);
    } catch {
      setErro(
        "Não foi possível conectar ao servidor de autenticação. Verifique sua conexão e tente novamente."
      );
    } finally {
      setCarregando(false);
    }
  };

  return (
    <div className="min-h-screen py-12 px-4 flex items-center justify-center font-body bg-[#f7f2e4] text-[#211d17]">
      <div className="max-w-md w-full border-2 border-[#2a251d] bg-[#fdfaf2] p-6 sm:p-8 shadow-2xl relative">
        {/* Detalhe de Moldura Cartorial */}
        <div className="text-center space-y-2 border-b border-[#c0b59d] pb-5">
          <div className="inline-flex items-center gap-1.5 px-2.5 py-0.5 border border-[#6fa878] bg-[#d7ebd9] text-[#153d1e] font-mono text-xs font-bold uppercase tracking-wider">
            <Vote className="w-3.5 h-3.5" />
            JÚRI POPULAR PARTIDÁRIO (ADR D1 & D2)
          </div>
          <h1 className="font-display font-black text-2xl sm:text-3xl uppercase tracking-tight text-[#211d17]">
            Portal do Filiado Votante
          </h1>
          <p className="text-xs text-[#6a6152] italic">
            Acesso reservado aos membros inscritos para habilitação de voto nos rankings Hype regionais e nacionais.
          </p>
        </div>

        {mensagemSucesso && (
          <div className="mt-4 p-3 border border-[#6fa878] bg-[#d7ebd9] text-[#153d1e] font-mono text-xs flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
            <span>{mensagemSucesso}</span>
          </div>
        )}

        {erro && (
          <div className="mt-4 p-3 border border-[#d48b84] bg-[#f8dcd9] text-[#5e1c18] font-mono text-xs flex items-center gap-2">
            <AlertCircle className="w-4 h-4 flex-shrink-0" />
            <span>{erro}</span>
          </div>
        )}

        <form onSubmit={handleValidarFiliado} className="mt-6 space-y-4 font-mono text-xs">
          <div>
            <label className="block font-bold uppercase text-[#211d17] mb-1">
              E-mail Cadastrado no Diretório *
            </label>
            <div className="relative">
              <Mail className="w-4 h-4 absolute left-3 top-2.5 text-[#6a6152]" />
              <input
                type="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="filiado@partido.org.br"
                className="w-full pl-9 pr-3 py-2 border border-[#2a251d] bg-[#fbf8ee] focus:outline-none focus:border-[#8f1f17] text-[#211d17]"
              />
            </div>
          </div>

          <div>
            <label className="block font-bold uppercase text-[#211d17] mb-1">
              Título de Eleitor ou CPF (Hash Auditável) *
            </label>
            <div className="relative">
              <Hash className="w-4 h-4 absolute left-3 top-2.5 text-[#6a6152]" />
              <input
                type="text"
                required
                value={identificador}
                onChange={(e) => setIdentificador(e.target.value)}
                placeholder="0000 0000 0000 ou 000.000.000-00"
                className="w-full pl-9 pr-3 py-2 border border-[#2a251d] bg-[#fbf8ee] focus:outline-none focus:border-[#8f1f17] text-[#211d17]"
              />
            </div>
            <p className="text-[10px] text-[#6a6152] mt-1 italic font-body">
              O documento é convertido em hash SHA-256 no backend. Nunca gravamos o CPF em texto puro.
            </p>
          </div>

          <button
            type="submit"
            disabled={carregando}
            className="w-full py-2.5 px-4 border-2 border-[#2a251d] bg-[#d7ebd9] hover:bg-[#6fa878] hover:text-white text-[#153d1e] font-bold uppercase tracking-wider transition-all flex items-center justify-center gap-2 shadow-sm disabled:opacity-50"
          >
            <span>{carregando ? "Validando Registro..." : "Habilitar Direito ao Voto"}</span>
            <ArrowRight className="w-4 h-4" />
          </button>
        </form>

        <div className="mt-8 pt-4 border-t border-[#c0b59d] flex flex-col sm:flex-row items-center justify-between gap-2 font-mono text-[11px] text-[#6a6152]">
          <Link href="/" className="underline hover:text-[#8f1f17]">
            ← Voltar ao Jornal Aberto
          </Link>
          <Link href="/auth/admin" className="text-[#8f1f17] font-bold hover:underline">
            Acesso Restrito Editorial / Adm →
          </Link>
        </div>
      </div>
    </div>
  );
}
