"use client";

import React, { useState } from "react";
import { Lock, Fingerprint, Smartphone, KeyRound, ShieldAlert } from "lucide-react";

export default function LoginPage() {
  const [email, setEmail] = useState("");
  const [senha, setSenha] = useState("");
  const [etapa2FA, setEtapa2FA] = useState(false);
  const [usuarioId, setUsuarioId] = useState<string | null>(null);
  const [desafio2faToken, setDesafio2faToken] = useState<string | null>(null);
  const [codigoTotp, setCodigoTotp] = useState("");
  const [metodo, setMetodo] = useState<"PASSKEY" | "GOOGLE_AUTH">("GOOGLE_AUTH");
  const [carregando, setCarregando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);

  const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";

  const handleLoginSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErro(null);
    setCarregando(true);

    try {
      const res = await fetch(`${apiUrl}/api/v1/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ email, senha }),
      });

      const data: {
        sucesso?: boolean;
        requer_2fa?: boolean;
        usuario_id?: string;
        desafio_2fa_token?: string | null;
        mensagem?: string;
      } | null = await res.json().catch(() => null);

      if (res.ok && data?.sucesso) {
        if (data.usuario_id) {
          setUsuarioId(data.usuario_id);
        }
        if (data.requer_2fa) {
          setDesafio2faToken(data.desafio_2fa_token || null);
          setEtapa2FA(true);
        } else {
          window.location.href = "/cms";
        }
      } else {
        setErro(data?.mensagem || "Credenciais inválidas.");
      }
    } catch {
      setErro("Falha de conexão com a API de segurança.");
    } finally {
      setCarregando(false);
    }
  };

  const handleValidar2FA = async (e: React.FormEvent) => {
    e.preventDefault();
    setErro(null);
    if (!usuarioId || !desafio2faToken) {
      setErro("Sessão de pré-autenticação inválida. Realize o login novamente.");
      setEtapa2FA(false);
      return;
    }

    setCarregando(true);
    try {
      const res = await fetch(`${apiUrl}/api/v1/auth/2fa/verify`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          usuario_id: usuarioId,
          codigo_totp: codigoTotp.trim(),
          desafio_2fa_token: desafio2faToken,
        }),
      });
      const data: {
        sucesso?: boolean;
        usuario_id?: string;
        mensagem?: string;
      } | null = await res.json().catch(() => null);

      if (!res.ok || !data?.sucesso) {
        setErro(data?.mensagem || "Código TOTP inválido.");
        return;
      }

      window.location.href = "/cms";
    } catch {
      setErro("Erro de comunicação ao validar o código 2FA.");
    } finally {
      setCarregando(false);
    }
  };

  const handlePasskeyAuth = async () => {
    if (!window.PublicKeyCredential) {
      setErro("Seu navegador ou dispositivo não possui suporte a Passkeys/Biometria.");
      return;
    }
    setErro(
      "O fluxo WebAuthn FIDO2 no servidor ainda não está pareado. Selecione 'Google Authenticator' para concluir o 2FA."
    );
  };

  return (
    <div className="min-h-[80vh] flex items-center justify-center px-4 py-12">
      <div className="max-w-md w-full bg-white p-8 rounded-2xl shadow-xl border border-slate-200 space-y-6">
        <div className="text-center space-y-2">
          <div className="w-12 h-12 bg-blue-50 text-destaque rounded-2xl flex items-center justify-center mx-auto shadow-sm">
            <Lock className="w-6 h-6" />
          </div>
          <h1 className="text-2xl font-black text-slate-900 tracking-tight">
            Autenticação Segura (RBAC)
          </h1>
          <p className="text-xs text-slate-500">
            Acesso reservado a redatores, lideranças e custodiantes com 2FA obrigatório.
          </p>
        </div>

        {erro && (
          <div className="bg-red-50 border border-red-200 text-red-700 text-xs p-3 rounded-lg flex items-center gap-2">
            <ShieldAlert className="w-4 h-4 flex-shrink-0" />
            <span>{erro}</span>
          </div>
        )}

        {!etapa2FA ? (
          <form onSubmit={handleLoginSubmit} className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1">
                E-mail Institucional
              </label>
              <input
                type="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="redator@jornalfederado.org.br"
                className="w-full px-3.5 py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 text-sm"
              />
            </div>

            <div>
              <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1">
                Senha de Acesso
              </label>
              <input
                type="password"
                required
                value={senha}
                onChange={(e) => setSenha(e.target.value)}
                placeholder="••••••••••••"
                className="w-full px-3.5 py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 text-sm"
              />
            </div>

            <button
              type="submit"
              className="w-full bg-destaque hover:bg-blue-700 text-white font-semibold py-2.5 rounded-lg text-sm shadow-sm transition-colors"
            >
              Avançar para 2FA &rarr;
            </button>
          </form>
        ) : (
          <div className="space-y-5">
            {/* Seletor de Método 2FA: Passkey vs Google Authenticator */}
            <div className="flex bg-slate-100 p-1 rounded-lg text-xs font-semibold">
              <button
                type="button"
                onClick={() => setMetodo("PASSKEY")}
                className={`flex-1 flex items-center justify-center gap-1.5 py-1.5 rounded-md transition-all ${
                  metodo === "PASSKEY" ? "bg-white text-destaque shadow-sm" : "text-slate-500"
                }`}
              >
                <Fingerprint className="w-4 h-4" />
                Passkey (Biometria)
              </button>
              <button
                type="button"
                onClick={() => setMetodo("GOOGLE_AUTH")}
                className={`flex-1 flex items-center justify-center gap-1.5 py-1.5 rounded-md transition-all ${
                  metodo === "GOOGLE_AUTH" ? "bg-white text-destaque shadow-sm" : "text-slate-500"
                }`}
              >
                <Smartphone className="w-4 h-4" />
                Google Authenticator
              </button>
            </div>

            {metodo === "PASSKEY" ? (
              <div className="text-center space-y-4 py-2">
                <p className="text-xs text-slate-600">
                  Toque no sensor biométrico ou use o FaceID do seu dispositivo para autenticar sem digitar códigos.
                </p>
                <button
                  type="button"
                  onClick={handlePasskeyAuth}
                  className="w-full flex items-center justify-center gap-2 bg-slate-900 hover:bg-slate-800 text-white font-semibold py-3 rounded-xl shadow-md transition-all"
                >
                  <Fingerprint className="w-5 h-5 text-emerald-400" />
                  Entrar com Biometria / Passkey
                </button>
              </div>
            ) : (
              <form
                onSubmit={handleValidar2FA}
                className="space-y-4"
              >
                <div>
                  <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1">
                    Código TOTP de 6 dígitos
                  </label>
                  <input
                    type="text"
                    required
                    maxLength={6}
                    value={codigoTotp}
                    onChange={(e) => setCodigoTotp(e.target.value)}
                    placeholder="123456"
                    className="w-full text-center tracking-widest font-mono text-xl py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900"
                  />
                </div>

                <button
                  type="submit"
                  disabled={carregando}
                  className="w-full bg-destaque hover:bg-blue-700 text-white font-semibold py-2.5 rounded-lg text-sm shadow-sm transition-colors disabled:opacity-50"
                >
                  {carregando ? "Validando..." : "Validar Código Google Authenticator"}
                </button>
              </form>
            )}

            <div className="text-center">
              <a href="#recovery" className="text-xs text-slate-500 hover:underline flex items-center justify-center gap-1">
                <KeyRound className="w-3.5 h-3.5" />
                Perdeu o autenticador? Usar código de recuperação (1 de 10)
              </a>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
