"use client";

import React, { useState } from "react";
import { Share2, Check, MessageCircle, Copy } from "lucide-react";

interface BotaoCompartilharProps {
  titulo: string;
}

export const BotaoCompartilhar: React.FC<BotaoCompartilharProps> = ({ titulo }) => {
  const [copiado, setCopiado] = useState(false);

  const handleCopiarLink = () => {
    if (typeof window !== "undefined") {
      navigator.clipboard.writeText(window.location.href);
      setCopiado(true);
      setTimeout(() => setCopiado(false), 2500);
    }
  };

  const handleWhatsApp = () => {
    if (typeof window !== "undefined") {
      const url = encodeURIComponent(window.location.href);
      const texto = encodeURIComponent(`*${titulo}*\n\nLeia a matéria completa no Jornal Digital Federado:\n${window.location.href}`);
      window.open(`https://api.whatsapp.com/send?text=${texto}`, "_blank");
    }
  };

  return (
    <div className="flex items-center gap-2">
      {/* Botão de Compartilhar no WhatsApp */}
      <button
        onClick={handleWhatsApp}
        className="flex items-center gap-1.5 bg-emerald-600 hover:bg-emerald-700 text-white px-3 py-1.5 rounded-lg text-xs font-semibold shadow-sm transition-all"
        title="Compartilhar no WhatsApp"
      >
        <MessageCircle className="w-4 h-4" />
        <span className="hidden sm:inline">WhatsApp</span>
      </button>

      {/* Botão de Copiar Link */}
      <button
        onClick={handleCopiarLink}
        className="flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all border border-slate-200"
        title="Copiar Link da Matéria"
      >
        {copiado ? (
          <>
            <Check className="w-4 h-4 text-emerald-600" />
            <span className="text-emerald-700">Copiado!</span>
          </>
        ) : (
          <>
            <Copy className="w-4 h-4" />
            <span className="hidden sm:inline">Copiar Link</span>
          </>
        )}
      </button>
    </div>
  );
};
