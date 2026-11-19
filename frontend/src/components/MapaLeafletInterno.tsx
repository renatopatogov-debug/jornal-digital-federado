"use client";

import React, { useEffect, useState } from "react";
import L from "leaflet";
import { MapContainer, TileLayer, GeoJSON, useMap } from "react-leaflet";

type GeoJsonPayload = NonNullable<Parameters<typeof L.geoJSON>[0]>;

interface ControladorCameraProps {
  ufSelecionada: string | null;
  ufGeoJson: unknown;
  municipioFocoGeoJson: unknown;
  resetCounter: number;
}

const ControladorCameraMapa: React.FC<ControladorCameraProps> = ({
  ufSelecionada,
  ufGeoJson,
  municipioFocoGeoJson,
  resetCounter,
}) => {
  const map = useMap();

  useEffect(() => {
    if (!ufSelecionada || !ufGeoJson) return;
    try {
      const layer = L.geoJSON(ufGeoJson as GeoJsonPayload);
      const bounds = layer.getBounds();
      if (bounds.isValid()) {
        map.fitBounds(bounds, { padding: [36, 36], maxZoom: 8, animate: true });
      }
    } catch {
      // Ignora geometria inválida mantendo câmera atual
    }
  }, [map, ufSelecionada, ufGeoJson]);

  useEffect(() => {
    if (!municipioFocoGeoJson) return;
    try {
      const layer = L.geoJSON(municipioFocoGeoJson as GeoJsonPayload);
      const bounds = layer.getBounds();
      if (bounds.isValid()) {
        map.fitBounds(bounds, { padding: [48, 48], maxZoom: 10, animate: true });
      }
    } catch {
      // Ignora geometria inválida
    }
  }, [map, municipioFocoGeoJson]);

  useEffect(() => {
    if (!ufSelecionada && resetCounter > 0) {
      map.setView([-14.235, -51.9253], 4, { animate: true });
    }
  }, [map, ufSelecionada, resetCounter]);

  return null;
};

export interface MapaLeafletInternoProps {
  brasilGeoJson: unknown;
  ufGeoJson: unknown;
  ufSelecionada: string | null;
  resetCounter: number;
  onSelectUf: (sigla: string, codigoIbge: string, nomeUf: string) => void;
  onSelectMunicipio: (codigoIbge: string, nome: string) => void;
}

export const MapaLeafletInterno: React.FC<MapaLeafletInternoProps> = ({
  brasilGeoJson,
  ufGeoJson,
  ufSelecionada,
  resetCounter,
  onSelectUf,
  onSelectMunicipio,
}) => {
  const [municipioFocoGeoJson, setMunicipioFocoGeoJson] = useState<unknown>(null);

  useEffect(() => {
    setMunicipioFocoGeoJson(null);
  }, [ufSelecionada, resetCounter]);

  return (
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

      <ControladorCameraMapa
        ufSelecionada={ufSelecionada}
        ufGeoJson={ufGeoJson}
        municipioFocoGeoJson={municipioFocoGeoJson}
        resetCounter={resetCounter}
      />

      {/* Camada das 27 UFs do Brasil (Nível Macro) */}
      {!ufSelecionada && Boolean(brasilGeoJson) && (
        <GeoJSON
          data={brasilGeoJson as GeoJsonPayload}
          style={() => ({
            fillColor: "#3b82f6",
            weight: 1.5,
            opacity: 1,
            color: "#60a5fa",
            fillOpacity: 0.25,
          })}
          onEachFeature={(feature, layer) => {
            const props = (feature.properties || {}) as {
              uf?: string;
              nome?: string;
              ibge_codigo?: string;
            };
            const uf = props.uf || "";
            const nome = props.nome || uf;
            const ibgeCodigo = props.ibge_codigo || uf;
            layer.bindTooltip(`<b>${nome} (${uf})</b><br/>Clique para explorar municípios`, {
              sticky: true,
            });
            layer.on({
              click: () => onSelectUf(uf, ibgeCodigo, nome),
              mouseover: (e) => (e.target as L.Path).setStyle({ fillOpacity: 0.6, weight: 2 }),
              mouseout: (e) => (e.target as L.Path).setStyle({ fillOpacity: 0.25, weight: 1.5 }),
            });
          }}
        />
      )}

      {/* Camada Municipal da UF Selecionada (Nível Micro sob Demanda) */}
      {ufSelecionada && Boolean(ufGeoJson) && (
        <GeoJSON
          key={ufSelecionada}
          data={ufGeoJson as GeoJsonPayload}
          style={() => ({
            fillColor: "#10b981",
            weight: 1,
            opacity: 0.9,
            color: "#34d399",
            fillOpacity: 0.35,
          })}
          onEachFeature={(feature, layer) => {
            const props = (feature.properties || {}) as {
              nome?: string;
              ibge_codigo?: string;
            };
            const nome = props.nome || "Município";
            const ibgeCodigo = props.ibge_codigo || "";
            layer.bindTooltip(`<b>${nome}</b><br/>Código IBGE: ${ibgeCodigo}`, {
              sticky: true,
            });
            layer.on({
              click: () => {
                setMunicipioFocoGeoJson(feature);
                onSelectMunicipio(ibgeCodigo, nome);
              },
              mouseover: (e) => (e.target as L.Path).setStyle({ fillOpacity: 0.7, weight: 2 }),
              mouseout: (e) => (e.target as L.Path).setStyle({ fillOpacity: 0.35, weight: 1 }),
            });
          }}
        />
      )}
    </MapContainer>
  );
};
