"use client";

import React, { useState, useMemo, useEffect, useRef } from "react";
import { MapaFederado } from "@/components/MapaFederado";
import { CardNoticia, NoticiaItem } from "@/components/CardNoticia";
import { BarraPesquisaTerritorio, FiltroFormato } from "@/components/BarraPesquisaTerritorio";
import { UF_POR_CODIGO_IBGE } from "@/lib/territorios";
import { Newspaper, Map, ChevronDown, ChevronUp, MapPin, FilterX, Loader2 } from "lucide-react";

interface PortalInterativoLeitorProps {
  noticiasIniciais: NoticiaItem[];
  modoEleitoralAtivo: boolean;
  temMaisInicial?: boolean;
}

const LIMITE_POR_PAGINA = 24;

function montarQueryFeed(params: {
  busca: string;
  ufAtiva: string | null;
  municipioAtivo: { codigo: string; nome: string } | null;
  formatoAtivo: FiltroFormato;
  offset: number;
}): string {
  const qs = new URLSearchParams();
  qs.set("limit", String(LIMITE_POR_PAGINA));
  qs.set("offset", String(params.offset));
  qs.set("incluir_nacional", "true");

  if (params.municipioAtivo) {
    qs.set("ibge", params.municipioAtivo.codigo);
  } else if (params.ufAtiva) {
    qs.set("uf", params.ufAtiva);
  }

  const termo = params.busca.trim();
  if (termo) {
    qs.set("q", termo);
  }

  if (params.formatoAtivo !== "TODOS") {
    qs.set("formato", params.formatoAtivo);
  }

  return qs.toString();
}

export const PortalInterativoLeitor: React.FC<PortalInterativoLeitorProps> = ({
  noticiasIniciais,
  modoEleitoralAtivo,
  temMaisInicial = false,
}) => {
  const [busca, setBusca] = useState("");
  const [ufAtiva, setUfAtiva] = useState<string | null>(null);
  const [municipioAtivo, setMunicipioAtivo] = useState<{ codigo: string; nome: string } | null>(null);
  const [formatoAtivo, setFormatoAtivo] = useState<FiltroFormato>("TODOS");
  const [mapaMobileAberto, setMapaMobileAberto] = useState(false);

  const [noticiasServidor, setNoticiasServidor] = useState<NoticiaItem[]>(noticiasIniciais);
  const [modoEleitoral, setModoEleitoral] = useState<boolean>(modoEleitoralAtivo);
  const [temMais, setTemMais] = useState<boolean>(temMaisInicial);
  const [carregandoFeed, setCarregandoFeed] = useState<boolean>(false);
  const [carregandoMais, setCarregandoMais] = useState<boolean>(false);
  const [usandoFallbackLocal, setUsandoFallbackLocal] = useState<boolean>(false);

  const primeiraRenderizacao = useRef<boolean>(true);
  const buscaAnterior = useRef<string>("");

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

  // Busca real no backend (/api/v1/feed) quando filtros ou pesquisa mudam
  useEffect(() => {
    if (primeiraRenderizacao.current) {
      primeiraRenderizacao.current = false;
      buscaAnterior.current = busca;
      return;
    }

    const buscaMudou = buscaAnterior.current !== busca;
    buscaAnterior.current = busca;

    const controller = new AbortController();
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    const atrasoMs = buscaMudou && busca.trim().length > 0 ? 300 : 0;

    const timer = setTimeout(async () => {
      setCarregandoFeed(true);
      try {
        const qs = montarQueryFeed({
          busca,
          ufAtiva,
          municipioAtivo,
          formatoAtivo,
          offset: 0,
        });
        const res = await fetch(`${apiUrl}/api/v1/feed?${qs}`, {
          signal: controller.signal,
        });

        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }

        const data: {
          modo_eleitoral_ativo?: boolean;
          tem_mais?: boolean;
          noticias?: NoticiaItem[];
        } = await res.json();

        setModoEleitoral(Boolean(data.modo_eleitoral_ativo));
        setNoticiasServidor(Array.isArray(data.noticias) ? data.noticias : []);
        setTemMais(Boolean(data.tem_mais));
        setUsandoFallbackLocal(false);
      } catch (err) {
        if ((err as { name?: string })?.name === "AbortError") {
          return;
        }
        // Fallback offline/demo quando o backend não está acessível
        setUsandoFallbackLocal(true);
        setTemMais(false);
      } finally {
        if (!controller.signal.aborted) {
          setCarregandoFeed(false);
        }
      }
    }, atrasoMs);

    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [busca, ufAtiva, municipioAtivo, formatoAtivo]);

  const handleCarregarMais = async () => {
    if (carregandoMais || !temMais || usandoFallbackLocal) return;
    setCarregandoMais(true);

    try {
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
      const qs = montarQueryFeed({
        busca,
        ufAtiva,
        municipioAtivo,
        formatoAtivo,
        offset: noticiasServidor.length,
      });
      const res = await fetch(`${apiUrl}/api/v1/feed?${qs}`);
      if (!res.ok) return;

      const data: {
        modo_eleitoral_ativo?: boolean;
        tem_mais?: boolean;
        noticias?: NoticiaItem[];
      } = await res.json();

      if (typeof data.modo_eleitoral_ativo === "boolean") {
        setModoEleitoral(data.modo_eleitoral_ativo);
      }
      setTemMais(Boolean(data.tem_mais));

      if (Array.isArray(data.noticias) && data.noticias.length > 0) {
        setNoticiasServidor((prev) => {
          const ids = new Set(prev.map((item) => item.id));
          const novos = data.noticias!.filter((item) => !ids.has(item.id));
          return [...prev, ...novos];
        });
      }
    } catch {
      // Mantém itens já carregados se houver falha momentânea de rede
    } finally {
      setCarregandoMais(false);
    }
  };

  // Fallback local aplicado apenas se a API estiver offline
  const noticiasFiltradas = useMemo(() => {
    if (!usandoFallbackLocal) {
      return noticiasServidor;
    }

    return noticiasIniciais.filter((n) => {
      if (formatoAtivo !== "TODOS" && (n.formato || "TEXTO") !== formatoAtivo) {
        return false;
      }

      if (municipioAtivo) {
        if (n.ibge_codigo !== municipioAtivo.codigo && n.escopo !== "NACIONAL") {
          return false;
        }
      } else if (ufAtiva) {
        const codigoUfDaNoticia = n.ibge_codigo ? n.ibge_codigo.slice(0, 2) : null;
        const siglaDaNoticia =
          n.uf || (codigoUfDaNoticia ? UF_POR_CODIGO_IBGE[codigoUfDaNoticia] : null);

        if (n.escopo !== "NACIONAL" && siglaDaNoticia !== ufAtiva) {
          return false;
        }
      }

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
  }, [usandoFallbackLocal, noticiasServidor, noticiasIniciais, formatoAtivo, municipioAtivo, ufAtiva, busca]);

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

      {/* Âncora do Mapa Brasil (#mapa) + Controle Mobile */}
      <div id="mapa" className="scroll-mt-20">
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
      </div>

      {/* Grid Principal: Feed + Coluna Lateral do Mapa no Desktop */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 items-start">
        <section className="lg:col-span-2 space-y-5">
          {/* Cabeçalho de Contexto Territorial */}
          <div className="flex items-center justify-between pb-3 border-b border-slate-200 flex-wrap gap-2">
            <div>
              <h1 className="text-xl md:text-2xl font-black text-slate-900 tracking-tight flex items-center gap-2">
                <Newspaper className="w-6 h-6 text-destaque" />
                {modoEleitoral
                  ? "Feed Oficial Cronológico (Lei nº 9.504/97)"
                  : "Feed Federado em Destaque"}
              </h1>
              <div className="flex items-center gap-2 text-xs text-slate-500 mt-1 flex-wrap">
                <span>
                  {modoEleitoral
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

            <span className="text-xs font-semibold bg-slate-100 text-slate-700 px-3 py-1 rounded-full inline-flex items-center gap-1.5">
              {carregandoFeed && <Loader2 className="w-3.5 h-3.5 animate-spin text-destaque" />}
              {noticiasFiltradas.length} {noticiasFiltradas.length === 1 ? "pauta" : "pautas"}
            </span>
          </div>

          {/* Lista de Notícias ou Estado Vazio Amigável */}
          {noticiasFiltradas.length > 0 ? (
            <>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                {noticiasFiltradas.map((noticia) => (
                  <CardNoticia
                    key={noticia.id}
                    noticia={noticia}
                    modoEleitoral={modoEleitoral}
                  />
                ))}
              </div>

              {temMais && !usandoFallbackLocal && (
                <div className="flex justify-center pt-2">
                  <button
                    type="button"
                    onClick={handleCarregarMais}
                    disabled={carregandoMais}
                    className="inline-flex items-center gap-2 bg-white hover:bg-slate-50 text-slate-800 border border-slate-300 font-semibold text-xs px-5 py-2.5 rounded-xl shadow-sm transition-colors disabled:opacity-60"
                  >
                    {carregandoMais && <Loader2 className="w-4 h-4 animate-spin text-destaque" />}
                    {carregandoMais ? "Carregando mais pautas..." : "Carregar mais pautas"}
                  </button>
                </div>
              )}
            </>
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
