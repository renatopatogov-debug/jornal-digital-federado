"use client";

import React, { useState, useMemo } from "react";
import { MapaFederado } from "@/components/MapaFederado";
import { CardNoticia, NoticiaItem } from "@/components/CardNoticia";
import { BarraPesquisaTerritorio, FiltroFormato } from "@/components/BarraPesquisaTerritorio";
import { Newspaper, Map, ChevronDown, ChevronUp, MapPin, FilterX } from "lucide-react";

interface PortalInterativoLeitorProps {
  noticiasIniciais: NoticiaItem[];
  modoEleitoralAtivo: boolean;
}

const UF_POR_CODIGO_IBGE: Record<string, string> = {
  "11": "RO", "12": "AC", "13": "AM", "14": "RR", "15": "PA", "16": "AP", "17": "TO",
  "21": "MA", "22": "PI", "23": "CE", "24": "RN", "25": "PB", "26": "PE", "27": "AL", "28": "SE", "29": "BA",
  "31": "MG", "32": "ES", "33": "RJ", "35": "SP",
  "41": "PR", "42": "SC", "43": "RS",
  "50": "MS", "51": "MT", "52": "GO", "53": "DF",
};

export const PortalInterativoLeitor: React.FC<PortalInterativoLeitorProps> = ({
  noticiasIniciais,
  modoEleitoralAtivo,
}) => {
  const [busca, setBusca] = useState("");
  const [ufAtiva, setUfAtiva] = useState<string | null>(null);
  const [municipioAtivo, setMunicipioAtivo] = useState<{ codigo: string; nome: string } | null>(null);
  const [formatoAtivo, setFormatoAtivo] = useState<FiltroFormato>("TODOS");
  const [mapaMobileAberto, setMapaMobileAberto] = useState(false);

  // Integração direta quando o leitor clica no Mapa Leaflet
  const handleSelectTerritorioMapa = (tipo: "UF" | "MUNICIPIO", codigo: string, nome: string) => {
    if (!codigo) {
      setUfAtiva(null);
      setMunicipioAtivo(null);
      return;
    }

    if (tipo === "UF") {
      const sigla = UF_POR_CODIGO_IBGE[codigo] || codigo.toUpperCase();
      setUfAtiva(sigla);
      setMunicipioAtivo(null);
    } else {
      const ufPrefix = codigo.slice(0, 2);
      const sigla = UF_POR_CODIGO_IBGE[ufPrefix] || ufAtiva;
      if (sigla) setUfAtiva(sigla);
      setMunicipioAtivo({ codigo, nome });
    }
  };

  // Filtragem reativa instantânea para o leitor
  const noticiasFiltradas = useMemo(() => {
    return noticiasIniciais.filter((n) => {
      // 1. Filtro por formato multimídia
      if (formatoAtivo !== "TODOS" && (n.formato || "TEXTO") !== formatoAtivo) {
        return false;
      }

      // 2. Filtro por município específico selecionado no mapa
      if (municipioAtivo) {
        if (n.ibge_codigo !== municipioAtivo.codigo && n.escopo !== "NACIONAL") {
          return false;
        }
      }
      // 3. Filtro por UF selecionada (inclui notícias estaduais da UF, municipais da UF e nacionais)
      else if (ufAtiva) {
        const codigoUfDaNoticia = n.ibge_codigo ? n.ibge_codigo.slice(0, 2) : null;
        const siglaDaNoticia =
          n.uf || (codigoUfDaNoticia ? UF_POR_CODIGO_IBGE[codigoUfDaNoticia] : null);

        if (n.escopo !== "NACIONAL" && siglaDaNoticia !== ufAtiva) {
          return false;
        }
      }

      // 4. Filtro por termo de busca textual
      if (busca.trim()) {
        const termo = busca.toLowerCase().trim();
        const matchTitulo = n.titulo.toLowerCase().includes(termo);
        const matchSubtitulo = (n.subtitulo || "").toLowerCase().includes(termo);
        const matchTerritorio = (n.territorio_nome || "").toLowerCase().includes(termo);
        const matchIbge = (n.ibge_codigo || "").toLowerCase().includes(termo);
        if (!matchTitulo && !matchSubtitulo && !matchTerritorio && !matchIbge) {
          return false;
        }
      }

      return true;
    });
  }, [noticiasIniciais, formatoAtivo, municipioAtivo, ufAtiva, busca]);

  return (
    <div className="space-y-6">
      {/* Barra de Busca, Format Tabs e Pills de UF */}
      <BarraPesquisaTerritorio
        busca={busca}
        ufAtiva={ufAtiva}
        municipioAtivo={municipioAtivo}
        formatoAtivo={formatoAtivo}
        onChangeBusca={setBusca}
        onSelectUf={(uf) => {
          setUfAtiva(uf);
          setMunicipioAtivo(null);
        }}
        onClearMunicipio={() => setMunicipioAtivo(null)}
        onSelectFormato={setFormatoAtivo}
      />

      {/* Botão Mobile para Abrir/Ocultar o Mapa sem empurrar o Feed */}
      <div className="lg:hidden">
        <button
          type="button"
          onClick={() => setMapaMobileAberto(!mapaMobileAberto)}
          className="w-full flex items-center justify-between bg-slate-900 text-white px-4 py-3 rounded-xl text-xs font-bold shadow-sm"
        >
          <span className="flex items-center gap-2">
            <Map className="w-4 h-4 text-blue-400" />
            {mapaMobileAberto
              ? "Ocultar Mapa Cartográfico do Brasil"
              : "Explorar Municípios no Mapa Interativo (27 UFs)"}
          </span>
          {mapaMobileAberto ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
        </button>

        {mapaMobileAberto && (
          <div className="mt-3">
            <MapaFederado onSelectTerritorio={handleSelectTerritorioMapa} />
          </div>
        )}
      </div>

      {/* Grid Principal: Feed + Coluna Lateral do Mapa no Desktop */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 items-start">
        <section className="lg:col-span-2 space-y-5">
          {/* Cabeçalho de Contexto Territorial */}
          <div className="flex items-center justify-between pb-3 border-b border-slate-200 flex-wrap gap-2">
            <div>
              <h1 className="text-xl md:text-2xl font-black text-slate-900 tracking-tight flex items-center gap-2">
                <Newspaper className="w-6 h-6 text-destaque" />
                {modoEleitoralAtivo
                  ? "Feed Oficial Cronológico (Lei nº 9.504/97)"
                  : "Feed Federado em Destaque"}
              </h1>
              <div className="flex items-center gap-2 text-xs text-slate-500 mt-1 flex-wrap">
                <span>
                  {modoEleitoralAtivo
                    ? "Ordenação estritamente cronológica."
                    : "Ranqueamento orgânico auditável pelo voto de filiados."}
                </span>
                {(ufAtiva || municipioAtivo) && (
                  <span className="inline-flex items-center gap-1 bg-blue-50 text-destaque font-bold px-2 py-0.5 rounded-md border border-blue-200">
                    <MapPin className="w-3 h-3" />
                    {municipioAtivo
                      ? `${municipioAtivo.nome} (${ufAtiva || "Município"})`
                      : `Estado: ${ufAtiva}`}
                  </span>
                )}
              </div>
            </div>

            <span className="text-xs font-semibold bg-slate-100 text-slate-700 px-3 py-1 rounded-full">
              {noticiasFiltradas.length} {noticiasFiltradas.length === 1 ? "pauta" : "pautas"}
            </span>
          </div>

          {/* Lista de Notícias ou Estado Vazio Amigável */}
          {noticiasFiltradas.length > 0 ? (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
              {noticiasFiltradas.map((noticia) => (
                <CardNoticia
                  key={noticia.id}
                  noticia={noticia}
                  modoEleitoral={modoEleitoralAtivo}
                />
              ))}
            </div>
          ) : (
            <div className="bg-white rounded-2xl border border-slate-200 p-10 text-center space-y-3">
              <FilterX className="w-10 h-10 text-slate-400 mx-auto" />
              <h3 className="text-base font-bold text-slate-800">
                Nenhuma matéria encontrada para este filtro
              </h3>
              <p className="text-xs text-slate-500 max-w-md mx-auto">
                Não localizamos pautas com os critérios selecionados. Limpe os filtros para visualizar todas as edições estaduais e nacionais.
              </p>
              <button
                type="button"
                onClick={() => {
                  setBusca("");
                  setUfAtiva(null);
                  setMunicipioAtivo(null);
                  setFormatoAtivo("TODOS");
                }}
                className="inline-flex items-center gap-1.5 bg-destaque hover:bg-blue-700 text-white text-xs font-semibold px-4 py-2 rounded-lg transition-colors"
              >
                Restaurar todas as notícias
              </button>
            </div>
          )}
        </section>

        {/* Mapa Lateral Fixo no Desktop */}
        <aside className="hidden lg:block space-y-3 lg:sticky lg:top-24">
          <div className="flex items-center justify-between">
            <h2 className="text-base font-bold text-slate-900 flex items-center gap-2">
              <Map className="w-4 h-4 text-destaque" />
              Navegação Cartográfica
            </h2>
            <span className="text-[11px] font-semibold text-slate-500 bg-slate-100 px-2 py-0.5 rounded">
              27 UFs • Sob Demanda
            </span>
          </div>
          <p className="text-xs text-slate-500">
            Clique em um estado para carregar os municípios e filtrar o feed ao lado instantaneamente.
          </p>
          <MapaFederado onSelectTerritorio={handleSelectTerritorioMapa} />
        </aside>
      </div>
    </div>
  );
};
