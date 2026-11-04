import React from "react";
import { AlertTriangle } from "lucide-react";

interface BannerEleitoralProps {
  ativo: boolean;
  mensagem?: string;
}

export const BannerEleitoral: React.FC<BannerEleitoralProps> = ({
  ativo,
  mensagem = "Exibição estritamente cronológica em observância à legislação eleitoral (Lei nº 9.504/97). Métricas e placares de engajamento Hype estão temporariamente suspensos.",
}) => {
  if (!ativo) return null;

  return (
    <div className="bg-red-800 text-white px-4 py-2.5 shadow-md border-b-2 border-red-900 sticky top-0 z-50">
      <div className="max-w-7xl mx-auto flex items-center justify-between gap-3 text-sm font-medium">
        <div className="flex items-center gap-2.5">
          <AlertTriangle className="w-5 h-5 flex-shrink-0 text-amber-300 animate-pulse" />
          <span>{mensagem}</span>
        </div>
        <span className="hidden md:inline-block bg-red-950 px-2 py-0.5 rounded text-xs text-red-200 uppercase tracking-wider font-semibold">
          Lei nº 9.504/97
        </span>
      </div>
    </div>
  );
};
