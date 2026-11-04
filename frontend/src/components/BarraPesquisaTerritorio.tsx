"use client";

import React from "react";
import { Search, MapPin, X, FileText, Video, Radio, Newspaper } from "lucide-react";

export type FiltroFormato = "TODOS" | "TEXTO" | "VIDEO" | "AUDIO" | "IMPRESSO_PDF";

interface BarraPesquisaTerritorioProps {
  busca: string;
  ufAtiva: string | null;
  municipioAtivo: { codigo: string; nome: string } | null;
  formatoAtivo: FiltroFormato;
  onChangeBusca: (termo: string) => void;
  onSelectUf: (uf: string | null) => void;
  onClearMunicipio: () => void;
  onSelectFormato: (formato: FiltroFormato) => void;
}

const UFS_RAPIDAS = [
  { uf: "SP", codigo: "35", nome: "São Paulo" },
  { uf: "RJ", codigo: "33", nome: "Rio de Janeiro" },
  { uf: "MG", codigo: "31", nome: "Minas Gerais" },
  { uf: "PR", codigo: "41", nome: "Paraná" },
  { uf: "BA", codigo: "29", nome: "Bahia" },
  { uf: "RS", codigo: "43", nome: "Rio Grande do Sul" },
  { uf: "DF", codigo: "53", nome: "Distrito Federal" },
  { uf: "SC", codigo: "42", nome: "Santa Catarina" },
  { uf: "PE", codigo: "26", nome: "Pernambuco" },
  { uf: "CE", codigo: "23", nome: "Ceará" },
];

export const BarraPesquisaTerritorio: React.FC<BarraPesquisaTerritorioProps> = ({
  busca,
  ufAtiva,
  municipioAtivo,
  formatoAtivo,
  onChangeBusca,
  onSelectUf,
  onClearMunicipio,
  onSelectFormato,
}) => {
  const temFiltroAtivo = Boolean(busca || ufAtiva || municipioAtivo || formatoAtivo !== "TODOS");

  const limparTudo = () => {
    onChangeBusca("");
    onSelectUf(null);
    onClearMunicipio();
    onSelectFormato("TODOS");
  };

  return (
    <div className="bg-white p-4 md:p-5 rounded-2xl shadow-sm border border-slate-200 space-y-4">
      <div className="flex flex-col md:flex-row gap-3">
        {/* Campo de Busca Textual */}
        <div className="relative flex-1">
          <Search className="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={busca}
            onChange={(e) => onChangeBusca(e.target.value)}
            placeholder="Buscar por cidade, estado, código IBGE ou assunto da pauta..."
            aria-label="Buscar notícias por município, estado ou assunto"
            className="w-full pl-10 pr-10 py-2.5 bg-slate-50 border border-slate-200 rounded-xl text-sm focus:outline-none focus:ring-2 focus:ring-destaque text-slate-800 placeholder-slate-400"
          />
          {busca && (
            <button
              type="button"
              onClick={() => onChangeBusca("")}
              aria-label="Limpar busca"
              className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        {/* Seletor de Formato Multimídia */}
        <div className="flex items-center gap-1 bg-slate-100 p-1 rounded-xl overflow-x-auto text-xs font-semibold">
          <button
            type="button"
            onClick={() => onSelectFormato("TODOS")}
            className={`px-3 py-1.5 rounded-lg transition-all flex items-center gap-1.5 flex-shrink-0 ${
              formatoAtivo === "TODOS"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            <Newspaper className="w-3.5 h-3.5" />
            Todos
          </button>
          <button
            type="button"
            onClick={() => onSelectFormato("VIDEO")}
            className={`px-3 py-1.5 rounded-lg transition-all flex items-center gap-1.5 flex-shrink-0 ${
              formatoAtivo === "VIDEO"
                ? "bg-red-600 text-white shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            <Video className="w-3.5 h-3.5" />
            Vídeos
          </button>
          <button
            type="button"
            onClick={() => onSelectFormato("AUDIO")}
            className={`px-3 py-1.5 rounded-lg transition-all flex items-center gap-1.5 flex-shrink-0 ${
              formatoAtivo === "AUDIO"
                ? "bg-purple-600 text-white shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            <Radio className="w-3.5 h-3.5" />
            Áudio
          </button>
          <button
            type="button"
            onClick={() => onSelectFormato("IMPRESSO_PDF")}
            className={`px-3 py-1.5 rounded-lg transition-all flex items-center gap-1.5 flex-shrink-0 ${
              formatoAtivo === "IMPRESSO_PDF"
                ? "bg-amber-600 text-white shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            <FileText className="w-3.5 h-3.5" />
            Paper News
          </button>
        </div>
      </div>

      {/* Atalhos Rápidos por UF & Breadcrumb Territorial */}
      <div className="flex items-center justify-between gap-2 flex-wrap">
        <div className="flex items-center gap-1.5 overflow-x-auto pb-1 text-xs">
          <button
            type="button"
            onClick={() => {
              onSelectUf(null);
              onClearMunicipio();
            }}
            className={`flex items-center gap-1 px-3 py-1.5 rounded-lg font-semibold transition-all flex-shrink-0 ${
              !ufAtiva && !municipioAtivo
                ? "bg-slate-900 text-white shadow-sm"
                : "bg-slate-100 text-slate-600 hover:bg-slate-200"
            }`}
          >
            <MapPin className="w-3.5 h-3.5" />
            Brasil Todo
          </button>

          {UFS_RAPIDAS.map((item) => (
            <button
              type="button"
              key={item.uf}
              onClick={() => onSelectUf(ufAtiva === item.uf ? null : item.uf)}
              title={`Filtrar notícias de ${item.nome}`}
              className={`px-3 py-1.5 rounded-lg font-bold transition-all flex-shrink-0 ${
                ufAtiva === item.uf
                  ? "bg-destaque text-white shadow-sm"
                  : "bg-slate-100 text-slate-600 hover:bg-slate-200"
              }`}
            >
              {item.uf}
            </button>
          ))}
        </div>

        {temFiltroAtivo && (
          <button
            type="button"
            onClick={limparTudo}
            className="text-xs font-semibold text-red-600 hover:text-red-700 flex items-center gap-1 px-2 py-1 rounded-md hover:bg-red-50 transition-colors"
          >
            <X className="w-3.5 h-3.5" />
            Limpar filtros
          </button>
        )}
      </div>
    </div>
  );
};
