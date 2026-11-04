"use client";

import React, { useEffect, useState } from "react";
import dynamic from "next/dynamic";
import { Compass, RotateCcw } from "lucide-react";

// Importação dinâmica para desabilitar SSR no Leaflet
const MapContainer = dynamic(
  () => import("react-leaflet").then((mod) => mod.MapContainer),
  { ssr: false }
);
const TileLayer = dynamic(
  () => import("react-leaflet").then((mod) => mod.TileLayer),
  { ssr: false }
);
const GeoJSON = dynamic(
  () => import("react-leaflet").then((mod) => mod.GeoJSON),
  { ssr: false }
);

interface MapaFederadoProps {
  onSelectTerritorio?: (tipo: "UF" | "MUNICIPIO", codigo: string, nome: string) => void;
}

export const MapaFederado: React.FC<MapaFederadoProps> = ({ onSelectTerritorio }) => {
  const [brasilGeoJson, setBrasilGeoJson] = useState<any>(null);
  const [ufGeoJson, setUfGeoJson] = useState<any>(null);
  const [ufSelecionada, setUfSelecionada] = useState<string | null>(null);
  const [carregando, setCarregando] = useState<boolean>(true);

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
    if (onSelectTerritorio) {
      onSelectTerritorio("UF", "", "Nacional");
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
            onClick={resetarParaNacional}
            className="flex items-center gap-1 bg-slate-700 hover:bg-slate-600 px-2 py-0.5 rounded text-[11px] transition-colors"
          >
            <RotateCcw className="w-3 h-3" />
            Ver Brasil
          </button>
        )}
      </div>

      {carregando && (
        <div className="absolute inset-0 z-[1001] bg-slate-950/60 backdrop-blur-sm flex items-center justify-center text-white text-sm font-medium">
          Carregando cartografia sob demanda...
        </div>
      )}

      {/* Container Leaflet */}
      {typeof window !== "undefined" && (
        <MapContainer
          center={[-14.235, -51.9253]}
          zoom={4}
          scrollWheelZoom={false}
          className="w-full h-full"
        >
          <TileLayer
            attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
            url="https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png"
          />

          {/* Camada das 27 UFs do Brasil (Nível Macro) */}
          {!ufSelecionada && brasilGeoJson && (
            <GeoJSON
              data={brasilGeoJson}
              style={() => ({
                fillColor: "#3b82f6",
                weight: 1.5,
                opacity: 1,
                color: "#60a5fa",
                fillOpacity: 0.25,
              })}
              onEachFeature={(feature: any, layer: any) => {
                const { uf, nome, ibge_codigo } = feature.properties;
                layer.bindTooltip(`<b>${nome} (${uf})</b><br/>Clique para explorar municípios`, {
                  sticky: true,
                });
                layer.on({
                  click: () => selecionarUf(uf, ibge_codigo, nome),
                  mouseover: (e: any) => e.target.setStyle({ fillOpacity: 0.6, weight: 2 }),
                  mouseout: (e: any) => e.target.setStyle({ fillOpacity: 0.25, weight: 1.5 }),
                });
              }}
            />
          )}

          {/* Camada Municipal da UF Selecionada (Nível Micro sob Demanda) */}
          {ufSelecionada && ufGeoJson && (
            <GeoJSON
              key={ufSelecionada}
              data={ufGeoJson}
              style={() => ({
                fillColor: "#10b981",
                weight: 1,
                opacity: 0.9,
                color: "#34d399",
                fillOpacity: 0.35,
              })}
              onEachFeature={(feature: any, layer: any) => {
                const { nome, ibge_codigo } = feature.properties;
                layer.bindTooltip(`<b>${nome}</b><br/>Código IBGE: ${ibge_codigo}`, {
                  sticky: true,
                });
                layer.on({
                  click: () => {
                    if (onSelectTerritorio) {
                      onSelectTerritorio("MUNICIPIO", ibge_codigo, nome);
                    }
                  },
                  mouseover: (e: any) => e.target.setStyle({ fillOpacity: 0.7, weight: 2 }),
                  mouseout: (e: any) => e.target.setStyle({ fillOpacity: 0.35, weight: 1 }),
                });
              }}
            />
          )}
        </MapContainer>
      )}
    </div>
  );
};
