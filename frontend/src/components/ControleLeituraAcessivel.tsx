"use client";

import React, { useState, useEffect } from "react";
import { Volume2, Square, Type, Sun, Moon, BookOpen } from "lucide-react";

interface ControleLeituraAcessivelProps {
  titulo: string;
  subtitulo?: string;
  conteudoHtmlSanitizado: string;
}

export const ControleLeituraAcessivel: React.FC<ControleLeituraAcessivelProps> = ({
  titulo,
  subtitulo,
  conteudoHtmlSanitizado,
}) => {
  const [escalaFonte, setEscalaFonte] = useState<"normal" | "grande" | "extra">("normal");
  const [temaLeitura, setTemaLeitura] = useState<"claro" | "sepia" | "escuro">("claro");
  const [lendoEmVozAlta, setLendoEmVozAlta] = useState(false);

  // Limpa síntese de voz ao sair da página
  useEffect(() => {
    return () => {
      if (typeof window !== "undefined" && "speechSynthesis" in window) {
        window.speechSynthesis.cancel();
      }
    };
  }, []);

  const alternarLeituraVoz = () => {
    if (typeof window === "undefined" || !("speechSynthesis" in window)) {
      alert("Seu navegador não suporta leitura em voz alta nativa.");
      return;
    }

    if (lendoEmVozAlta) {
      window.speechSynthesis.cancel();
      setLendoEmVozAlta(false);
      return;
    }

    const textoPuro = `${titulo}. ${subtitulo || ""}. ${conteudoHtmlSanitizado.replace(/<[^>]+>/g, " ")}`;
    const utterance = new SpeechSynthesisUtterance(textoPuro);
    utterance.lang = "pt-BR";
    utterance.rate = 1.05;
    utterance.onend = () => setLendoEmVozAlta(false);
    utterance.onerror = () => setLendoEmVozAlta(false);

    window.speechSynthesis.cancel();
    window.speechSynthesis.speak(utterance);
    setLendoEmVozAlta(true);
  };

  const classesTamanho = {
    normal: "text-base md:text-lg leading-relaxed",
    grande: "text-lg md:text-xl leading-relaxed",
    extra: "text-xl md:text-2xl leading-loose",
  }[escalaFonte];

  const classesTema = {
    claro: "bg-white text-slate-800 border-transparent",
    sepia: "bg-amber-50/70 text-amber-950 p-6 md:p-8 rounded-2xl border border-amber-200",
    escuro: "bg-slate-900 text-slate-100 p-6 md:p-8 rounded-2xl border border-slate-800",
  }[temaLeitura];

  return (
    <div className="space-y-6">
      {/* Barra de Acessibilidade e Conforto Visual */}
      <div className="flex items-center justify-between gap-3 flex-wrap bg-slate-100/90 px-4 py-2.5 rounded-xl border border-slate-200 text-xs">
        {/* Botão Ouvir Matéria (Text-to-Speech Nativo) */}
        <button
          type="button"
          onClick={alternarLeituraVoz}
          className={`flex items-center gap-2 px-3.5 py-1.5 rounded-lg font-bold transition-all shadow-sm ${
            lendoEmVozAlta
              ? "bg-red-600 text-white animate-pulse"
              : "bg-destaque hover:bg-blue-700 text-white"
          }`}
        >
          {lendoEmVozAlta ? (
            <>
              <Square className="w-3.5 h-3.5 fill-white" />
              Parar Leitura em Voz
            </>
          ) : (
            <>
              <Volume2 className="w-4 h-4" />
              Ouvir Notícia (Áudio IA/Voz)
            </>
          )}
        </button>

        {/* Controles de Fonte e Tema de Leitura */}
        <div className="flex items-center gap-3 flex-wrap">
          <div className="flex items-center gap-1 bg-white px-2 py-1 rounded-lg border border-slate-200">
            <Type className="w-3.5 h-3.5 text-slate-400 mr-1" />
            <button
              type="button"
              onClick={() => setEscalaFonte("normal")}
              className={`px-2 py-0.5 rounded font-bold ${
                escalaFonte === "normal" ? "bg-slate-900 text-white" : "text-slate-600 hover:bg-slate-100"
              }`}
              title="Fonte padrão"
            >
              A
            </button>
            <button
              type="button"
              onClick={() => setEscalaFonte("grande")}
              className={`px-2 py-0.5 rounded font-bold text-sm ${
                escalaFonte === "grande" ? "bg-slate-900 text-white" : "text-slate-600 hover:bg-slate-100"
              }`}
              title="Fonte ampliada"
            >
              A+
            </button>
            <button
              type="button"
              onClick={() => setEscalaFonte("extra")}
              className={`px-2 py-0.5 rounded font-bold text-base ${
                escalaFonte === "extra" ? "bg-slate-900 text-white" : "text-slate-600 hover:bg-slate-100"
              }`}
              title="Fonte extra grande (acessibilidade)"
            >
              A++
            </button>
          </div>

          <div className="flex items-center gap-1 bg-white px-2 py-1 rounded-lg border border-slate-200">
            <button
              type="button"
              onClick={() => setTemaLeitura("claro")}
              className={`flex items-center gap-1 px-2 py-0.5 rounded font-semibold ${
                temaLeitura === "claro" ? "bg-slate-900 text-white" : "text-slate-600"
              }`}
            >
              <Sun className="w-3 h-3" /> Claro
            </button>
            <button
              type="button"
              onClick={() => setTemaLeitura("sepia")}
              className={`flex items-center gap-1 px-2 py-0.5 rounded font-semibold ${
                temaLeitura === "sepia" ? "bg-amber-800 text-white" : "text-amber-900"
              }`}
            >
              <BookOpen className="w-3 h-3" /> Sépia
            </button>
            <button
              type="button"
              onClick={() => setTemaLeitura("escuro")}
              className={`flex items-center gap-1 px-2 py-0.5 rounded font-semibold ${
                temaLeitura === "escuro" ? "bg-slate-800 text-white" : "text-slate-600"
              }`}
            >
              <Moon className="w-3 h-3" /> Noturno
            </button>
          </div>
        </div>
      </div>

      {/* Corpo Editorial Sanitizado com Escala de Fonte e Tema Selecionado */}
      <div
        className={`transition-all duration-200 space-y-4 ${classesTamanho} ${classesTema} [&_h2]:text-2xl [&_h2]:font-bold [&_h2]:mt-6 [&_h2]:mb-2 [&_blockquote]:border-l-4 [&_blockquote]:border-destaque [&_blockquote]:pl-4 [&_blockquote]:italic [&_blockquote]:my-4 [&_ul]:list-disc [&_ul]:pl-6 [&_a]:text-destaque [&_a]:underline`}
        dangerouslySetInnerHTML={{ __html: conteudoHtmlSanitizado }}
      />
    </div>
  );
};
