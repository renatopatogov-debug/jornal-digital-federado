"use client";

import React, { useState } from "react";
import Link from "next/link";
import { Lock, Fingerprint, ShieldAlert, ArrowRight, Building2, MapPin, Crown, AlertCircle } from "lucide-react";

export default function LoginAdminPage() {
  const [email, setEmail] = useState("");
  const [senha, setSenha] = useState("");
  const [jurisdicaoSelecionada, setJurisdicaoSelecionada] = useState<"MUNICIPAL" | "ESTADUAL" | "PRESIDENCIAL">("MUNICIPAL");
  const [etapa2FA, setEtapa2FA] = useState(false);
  const [codigoTotp, setCodigoTotp] = useState("");
  const [carregando, setCarregando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);

  const handleLoginSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErro(null);
    setCarregando(true);

    try {
      // Simulação / Chamada à API oficial em Rust
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
      const res = await fetch(`${apiUrl}/api/v1/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, senha, jurisdicao: jurisdicaoSelecionada }),
      }).catch(() => null);

      if (res && res.ok) {
        const data = await res.json();
        if (data.requer_2fa) {
          setEtapa2FA(true);
        } else {
          redirecionarPorCargo(jurisdicaoSelecionada);
        }
      } else {
        // Modo demonstração local controlado
        setEtapa2FA(true);
      }
    } catch {
      setErro("Falha de conexão com a infraestrutura de segurança editorial.");
    } finally {
      setCarregando(false);
    }
  };

  const handleValidar2FA = (e: React.FormEvent) => {
    e.preventDefault();
    if (codigoTotp.length < 6) {
      setErro("Informe os 6 dígitos do autenticador.");
      return;
    }
    redirecionarPorCargo(jurisdicaoSelecionada);
  };

  const handlePasskeyAuth = () => {
    // Simulação do desafio biométrico FIDO2/WebAuthn
    alert(`Autenticando via Passkey / Biometria com credencial de ${jurisdicaoSelecionada}...`);
    redirecionarPorCargo(jurisdicaoSelecionada);
  };

  const redirecionarPorCargo = (jurisdicao: "MUNICIPAL" | "ESTADUAL" | "PRESIDENCIAL") => {
    if (jurisdicao === "PRESIDENCIAL") {
      window.location.href = "/cms?visao=presidencia";
    } else if (jurisdicao === "ESTADUAL") {
      window.location.href = "/cms?visao=escalonamento";
    } else {
      window.location.href = "/cms";
    }
  };

  return (
    <div className="min-h-screen py-12 px-4 flex items-center justify-center font-body bg-[#ece5d3] text-[#211d17]">
      <div className="max-w-md w-full border-2 border-[#2a251d] bg-[#f7f2e4] p-6 sm:p-8 shadow-2xl relative">
        <div className="text-center space-y-2 border-b border-[#c0b59d] pb-5">
          <div className="inline-flex items-center gap-1.5 px-2.5 py-0.5 border border-[#8f1f17] bg-[#f8dcd9] text-[#5e1c18] font-mono text-xs font-bold uppercase tracking-wider">
            <Lock className="w-3.5 h-3.5" />
            CHANCELARIA • ANEL DE CONFIANÇA 2FA (ADR D4)
          </div>
          <h1 className="font-display font-black text-2xl sm:text-3xl uppercase tracking-tight text-[#211d17]">
            Mesa de Redação & Adm
          </h1>
          <p className="text-xs text-[#6a6152] italic">
            Acesso exclusivo a redatores, lideranças municipais, regionais e executiva nacional.
          </p>
        </div>

        {erro && (
          <div className="mt-4 p-3 border border-[#d48b84] bg-[#f8dcd9] text-[#5e1c18] font-mono text-xs flex items-center gap-2">
            <AlertCircle className="w-4 h-4 flex-shrink-0" />
            <span>{erro}</span>
          </div>
        )}

        {!etapa2FA ? (
          <form onSubmit={handleLoginSubmit} className="mt-6 space-y-4 font-mono text-xs">
            {/* Seletor de Nível de Jurisdição do Administrador */}
            <div>
              <label className="block font-bold uppercase text-[#211d17] mb-1">
                Alçada de Jurisdição Territorial *
              </label>
              <div className="grid grid-cols-3 gap-1 border border-[#2a251d] p-1 bg-[#fbf8ee]">
                <button
                  type="button"
                  onClick={() => setJurisdicaoSelecionada("MUNICIPAL")}
                  className={`p-1.5 text-center font-bold flex flex-col items-center gap-1 transition-all ${
                    jurisdicaoSelecionada === "MUNICIPAL"
                      ? "bg-[#2a251d] text-[#f7f2e4]"
                      : "text-[#6a6152] hover:text-[#211d17]"
                  }`}
                >
                  <MapPin className="w-3.5 h-3.5" />
                  <span className="text-[10px]">Municipal</span>
                </button>
                <button
                  type="button"
                  onClick={() => setJurisdicaoSelecionada("ESTADUAL")}
                  className={`p-1.5 text-center font-bold flex flex-col items-center gap-1 transition-all ${
                    jurisdicaoSelecionada === "ESTADUAL"
                      ? "bg-[#2a251d] text-[#f7f2e4]"
                      : "text-[#6a6152] hover:text-[#211d17]"
                  }`}
                >
                  <Building2 className="w-3.5 h-3.5" />
                  <span className="text-[10px]">Estadual</span>
                </button>
                <button
                  type="button"
                  onClick={() => setJurisdicaoSelecionada("PRESIDENCIAL")}
                  className={`p-1.5 text-center font-bold flex flex-col items-center gap-1 transition-all ${
                    jurisdicaoSelecionada === "PRESIDENCIAL"
                      ? "bg-[#2a251d] text-[#f7f2e4]"
                      : "text-[#6a6152] hover:text-[#211d17]"
                  }`}
                >
                  <Crown className="w-3.5 h-3.5" />
                  <span className="text-[10px]">Nacional</span>
                </button>
              </div>
            </div>

            <div>
              <label className="block font-bold uppercase text-[#211d17] mb-1">
                E-mail Institucional ou Credencial *
              </label>
              <input
                type="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="redator.comarca@partido.org.br"
                className="w-full px-3 py-2 border border-[#2a251d] bg-[#fbf8ee] focus:outline-none focus:border-[#8f1f17] text-[#211d17]"
              />
            </div>

            <div>
              <label className="block font-bold uppercase text-[#211d17] mb-1">
                Senha Segura de Protocolo *
              </label>
              <input
                type="password"
                required
                value={senha}
                onChange={(e) => setSenha(e.target.value)}
                placeholder="••••••••••••"
                className="w-full px-3 py-2 border border-[#2a251d] bg-[#fbf8ee] focus:outline-none focus:border-[#8f1f17] text-[#211d17]"
              />
            </div>

            <button
              type="submit"
              disabled={carregando}
              className="w-full py-2.5 px-4 border-2 border-[#2a251d] bg-[#2a251d] hover:bg-[#8f1f17] text-[#f7f2e4] font-bold uppercase tracking-wider transition-all flex items-center justify-center gap-2 shadow-sm disabled:opacity-50"
            >
              <span>{carregando ? "Verificando Assinatura..." : "Avançar para Desafio 2FA"}</span>
              <ArrowRight className="w-4 h-4" />
            </button>

            <div className="relative flex py-2 items-center">
              <div className="flex-grow border-t border-[#c0b59d]"></div>
              <span className="flex-shrink mx-2 text-[10px] text-[#6a6152] uppercase">ou autenticação por hardware</span>
              <div className="flex-grow border-t border-[#c0b59d]"></div>
            </div>

            <button
              type="button"
              onClick={handlePasskeyAuth}
              className="w-full py-2 px-4 border border-[#2a251d] bg-[#fbf8ee] hover:bg-[#efe8d6] font-bold uppercase text-[#211d17] flex items-center justify-center gap-2"
            >
              <Fingerprint className="w-4 h-4 text-[#8f1f17]" />
              <span>Entrar com Passkey / Biometria FIDO2</span>
            </button>
          </form>
        ) : (
          <form onSubmit={handleValidar2FA} className="mt-6 space-y-4 font-mono text-xs">
            <div className="p-3 border border-[#2a251d] bg-[#faeed4] text-[#573c09]">
              <span className="font-bold uppercase block mb-1">Segundo Fator Requerido</span>
              <p className="text-[11px] font-body">
                Insira o código de 6 dígitos gerado pelo seu aplicativo autenticador (Google Authenticator / YubiKey).
              </p>
            </div>

            <div>
              <label className="block font-bold uppercase text-[#211d17] mb-1 text-center">
                Código TOTP (6 dígitos)
              </label>
              <input
                type="text"
                maxLength={6}
                autoFocus
                required
                value={codigoTotp}
                onChange={(e) => setCodigoTotp(e.target.value.replace(/\D/g, ""))}
                placeholder="000 000"
                className="w-full px-3 py-2.5 border-2 border-[#2a251d] bg-[#fbf8ee] text-center font-bold text-lg tracking-[0.3em] focus:outline-none focus:border-[#8f1f17]"
              />
            </div>

            <button
              type="submit"
              className="w-full py-2.5 px-4 border-2 border-[#2a251d] bg-[#2a251d] hover:bg-[#8f1f17] text-[#f7f2e4] font-bold uppercase tracking-wider transition-all"
            >
              Validar Protocolo e Abrir Mesa
            </button>

            <button
              type="button"
              onClick={() => setEtapa2FA(false)}
              className="w-full text-center text-[11px] underline text-[#6a6152] hover:text-[#211d17]"
            >
              ← Voltar e trocar credencial
            </button>
          </form>
        )}

        <div className="mt-8 pt-4 border-t border-[#c0b59d] flex flex-col sm:flex-row items-center justify-between gap-2 font-mono text-[11px] text-[#6a6152]">
          <Link href="/" className="underline hover:text-[#8f1f17]">
            ← Voltar ao Feed Geral
          </Link>
          <Link href="/auth/filiado" className="hover:underline">
            Sou apenas Filiado Votante →
          </Link>
        </div>
      </div>
    </div>
  );
}
