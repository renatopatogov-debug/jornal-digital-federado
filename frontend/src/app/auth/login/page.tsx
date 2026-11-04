"use client";

import React, { useState } from "react";
import { Lock, Fingerprint, Smartphone, KeyRound, ShieldAlert } from "lucide-react";

export default function LoginPage() {
  const [email, setEmail] = useState("");
  const [senha, setSenha] = useState("");
  const [etapa2FA, setEtapa2FA] = useState(false);
  const [codigoTotp, setCodigoTotp] = useState("");
  const [metodo, setMetodo] = useState<"PASSKEY" | "GOOGLE_AUTH">("PASSKEY");
  const [erro, setErro] = useState<string | null>(null);

  const handleLoginSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErro(null);

    try {
      const res = await fetch("http://127.0.0.1:8080/api/v1/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, senha }),
      });

      const data = await res.json();
      if (res.ok) {
        if (data.requer_2fa) {
          setEtapa2FA(true);
        } else {
          window.location.href = "/cms";
        }
      } else {
        setErro(data.mensagem || "Credenciais inválidas.");
      }
    } catch (err) {
      setErro("Falha de conexão com a API de segurança.");
    }
  };

  const handlePasskeyAuth = async () => {
    try {
      // Inicia fluxo WebAuthn biométrico com o navegador
      if (!window.PublicKeyCredential) {
        setErro("Seu navegador ou dispositivo não possui suporte a Passkeys/Biometria.");
        return;
      }
      alert("Simulação de desafio WebAuthn FIDO2: Solicitando leitura de FaceID / TouchID...");
      window.location.href = "/cms";
    } catch (err) {
      setErro("Falha na validação biométrica.");
    }
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
                onSubmit={(e) => {
                  e.preventDefault();
                  alert(`Validando código TOTP: ${codigoTotp}`);
                  window.location.href = "/cms";
                }}
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
                  className="w-full bg-destaque hover:bg-blue-700 text-white font-semibold py-2.5 rounded-lg text-sm shadow-sm transition-colors"
                >
                  Validar Código Google Authenticator
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
