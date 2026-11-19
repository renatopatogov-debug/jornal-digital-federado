"use client";

import React, { useEffect, useState } from "react";
import dynamic from "next/dynamic";
import { Compass, RotateCcw } from "lucide-react";

const MapaLeafletInterno = dynamic(
  () => import("@/components/MapaLeafletInterno").then((mod) => mod.MapaLeafletInterno),
  { ssr: false }
);

interface MapaFederadoProps {
  onSelectTerritorio?: (tipo: "UF" | "MUNICIPIO", codigo: string, nome: string) => void;
}

export const MapaFederado: React.FC<MapaFederadoProps> = ({ onSelectTerritorio }) => {
  const [montado, setMontado] = useState<boolean>(false);
  const [brasilGeoJson, setBrasilGeoJson] = useState<unknown>(null);
  const [ufGeoJson, setUfGeoJson] = useState<unknown>(null);
  const [ufSelecionada, setUfSelecionada] = useState<string | null>(null);
  const [carregando, setCarregando] = useState<boolean>(true);
  const [resetCounter, setResetCounter] = useState<number>(0);

  // Evita mismatch de hidratação SSR no Next.js 14
  useEffect(() => {
    setMontado(true);
  }, []);

  // 1. Carregar malha macro do Brasil (27 UFs)
  useEffect(() => {
    fetch("/geo/brasil_ufs.json")
      .then((res) => res.json())
      .then((data) => {
        setBrasilGeoJson(data);
        setCarregando(false);
      })
      .catch((err) => {
        console.error("Falha ao carregar malha do Brasil:", err);
        setCarregando(false);
      });
  }, []);

  // 2. Carregar sob demanda a malha municipal da UF selecionada (< 250 KB)
  const selecionarUf = async (sigla: string, codigoIbge: string, nomeUf: string) => {
    setUfSelecionada(sigla);
    setCarregando(true);
    if (onSelectTerritorio) {
      onSelectTerritorio("UF", codigoIbge, nomeUf);
    }

    try {
      const res = await fetch(`/geo/ufs/uf_${sigla.toLowerCase()}.json`);
      if (res.ok) {
        const data = await res.json();
        setUfGeoJson(data);
      }
    } catch (err) {
      console.error(`Erro ao carregar UF ${sigla}:`, err);
    } finally {
      setCarregando(false);
    }
  };

  const resetarParaNacional = () => {
    setUfSelecionada(null);
    setUfGeoJson(null);
    setResetCounter((c) => c + 1);
    if (onSelectTerritorio) {
      onSelectTerritorio("UF", "", "Nacional");
    }
  };

  const selecionarMunicipio = (codigoIbge: string, nome: string) => {
    if (onSelectTerritorio) {
      onSelectTerritorio("MUNICIPIO", codigoIbge, nome);
    }
  };

  return (
    <div className="relative w-full h-[460px] bg-slate-900 rounded-2xl overflow-hidden border border-slate-700 shadow-xl">
      {/* Barra de Controle de Camadas */}
      <div className="absolute top-4 left-4 z-[1000] bg-slate-900/90 backdrop-blur-md px-4 py-2 rounded-lg border border-slate-700 text-white flex items-center gap-3 text-xs shadow-md">
        <Compass className="w-4 h-4 text-destaque animate-spin-slow" />
        <span className="font-semibold">
          {ufSelecionada ? `Visualizando Estado: ${ufSelecionada}` : "Navegação Federada — 27 UFs"}
        </span>
        {ufSelecionada && (
          <button
            type="button"
            onClick={resetarParaNacional}
            className="flex items-center gap-1 bg-slate-700 hover:bg-slate-600 px-2 py-0.5 rounded text-[11px] transition-colors"
          >
            <RotateCcw className="w-3 h-3" />
            Ver Brasil
          </button>
        )}
      </div>

      {(!montado || carregando) && (
        <div className="absolute inset-0 z-[1001] bg-slate-950/60 backdrop-blur-sm flex items-center justify-center text-white text-sm font-medium">
          Carregando cartografia sob demanda...
        </div>
      )}

      {/* Container Leaflet montado estritamente após hidratação no cliente */}
      {montado && (
        <MapaLeafletInterno
          brasilGeoJson={brasilGeoJson}
          ufGeoJson={ufGeoJson}
          ufSelecionada={ufSelecionada}
          resetCounter={resetCounter}
          onSelectUf={selecionarUf}
          onSelectMunicipio={selecionarMunicipio}
        />
      )}
    </div>
  );
};
