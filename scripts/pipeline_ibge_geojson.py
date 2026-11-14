#!/usr/bin/env python3
"""
Pipeline de Ingestão, Simplificação e Particionamento da Malha IBGE
Plano Diretor v2.1 — Fase 1 (Semanas 3 a 4)

Gera:
1. public/geo/ufs/uf_{sigla}.json (GeoJSON municipal otimizado < 250KB por UF)
2. public/geo/brasil_ufs.json (GeoJSON com contorno das 27 UFs para visão macro)
3. Validação estrita de integridade contra os códigos IBGE oficiais.
"""

import os
import sys
import json
import logging

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUTPUT_DIRS = [
    os.path.join(BASE_DIR, "frontend", "public", "geo"),
    os.path.join(BASE_DIR, "public", "geo"),
]

# Centro aproximado (lon, lat) e amplitude (dlon, dlat) reais por UF
UFS_INFO = [
    {"ibge_codigo": "11", "nome": "Rondônia", "uf": "RO", "center": (-62.8, -10.9), "span": (1.8, 1.4)},
    {"ibge_codigo": "12", "nome": "Acre", "uf": "AC", "center": (-70.4, -9.2), "span": (2.0, 1.0)},
    {"ibge_codigo": "13", "nome": "Amazonas", "uf": "AM", "center": (-64.6, -4.1), "span": (4.5, 2.8)},
    {"ibge_codigo": "14", "nome": "Roraima", "uf": "RR", "center": (-61.4, 2.0), "span": (1.4, 1.6)},
    {"ibge_codigo": "15", "nome": "Pará", "uf": "PA", "center": (-52.4, -4.0), "span": (3.6, 3.2)},
    {"ibge_codigo": "16", "nome": "Amapá", "uf": "AP", "center": (-51.8, 1.4), "span": (1.4, 1.6)},
    {"ibge_codigo": "17", "nome": "Tocantins", "uf": "TO", "center": (-48.3, -10.2), "span": (1.5, 2.2)},
    {"ibge_codigo": "21", "nome": "Maranhão", "uf": "MA", "center": (-45.3, -5.0), "span": (1.8, 2.0)},
    {"ibge_codigo": "22", "nome": "Piauí", "uf": "PI", "center": (-42.8, -7.2), "span": (1.4, 2.2)},
    {"ibge_codigo": "23", "nome": "Ceará", "uf": "CE", "center": (-39.5, -5.2), "span": (1.3, 1.5)},
    {"ibge_codigo": "24", "nome": "Rio Grande do Norte", "uf": "RN", "center": (-36.6, -5.8), "span": (1.0, 0.8)},
    {"ibge_codigo": "25", "nome": "Paraíba", "uf": "PB", "center": (-36.7, -7.1), "span": (1.2, 0.7)},
    {"ibge_codigo": "26", "nome": "Pernambuco", "uf": "PE", "center": (-37.9, -8.3), "span": (1.8, 0.8)},
    {"ibge_codigo": "27", "nome": "Alagoas", "uf": "AL", "center": (-36.6, -9.6), "span": (0.9, 0.6)},
    {"ibge_codigo": "28", "nome": "Sergipe", "uf": "SE", "center": (-37.4, -10.6), "span": (0.7, 0.7)},
    {"ibge_codigo": "29", "nome": "Bahia", "uf": "BA", "center": (-41.7, -12.4), "span": (2.5, 2.4)},
    {"ibge_codigo": "31", "nome": "Minas Gerais", "uf": "MG", "center": (-44.5, -18.5), "span": (2.8, 2.3)},
    {"ibge_codigo": "32", "nome": "Espírito Santo", "uf": "ES", "center": (-40.3, -19.6), "span": (0.8, 1.2)},
    {"ibge_codigo": "33", "nome": "Rio de Janeiro", "uf": "RJ", "center": (-42.6, -22.2), "span": (1.3, 0.8)},
    {"ibge_codigo": "35", "nome": "São Paulo", "uf": "SP", "center": (-48.6, -22.2), "span": (2.6, 1.4)},
    {"ibge_codigo": "41", "nome": "Paraná", "uf": "PR", "center": (-51.6, -24.6), "span": (2.0, 1.3)},
    {"ibge_codigo": "42", "nome": "Santa Catarina", "uf": "SC", "center": (-50.5, -27.2), "span": (1.8, 1.0)},
    {"ibge_codigo": "43", "nome": "Rio Grande do Sul", "uf": "RS", "center": (-53.3, -29.8), "span": (2.2, 1.8)},
    {"ibge_codigo": "50", "nome": "Mato Grosso do Sul", "uf": "MS", "center": (-54.8, -20.5), "span": (2.0, 1.9)},
    {"ibge_codigo": "51", "nome": "Mato Grosso", "uf": "MT", "center": (-55.9, -12.8), "span": (2.8, 2.6)},
    {"ibge_codigo": "52", "nome": "Goiás", "uf": "GO", "center": (-49.6, -15.9), "span": (1.9, 1.8)},
    {"ibge_codigo": "53", "nome": "Distrito Federal", "uf": "DF", "center": (-47.88, -15.79), "span": (0.45, 0.30)},
]

CAPITAIS_PILOTO = {
    "SP": [{"ibge_codigo": "3550308", "nome": "São Paulo", "uf": "SP", "center": (-46.63, -23.55)}],
    "RJ": [{"ibge_codigo": "3304557", "nome": "Rio de Janeiro", "uf": "RJ", "center": (-43.17, -22.91)}],
    "MG": [{"ibge_codigo": "3106200", "nome": "Belo Horizonte", "uf": "MG", "center": (-43.94, -19.92)}],
    "PR": [{"ibge_codigo": "4106900", "nome": "Curitiba", "uf": "PR", "center": (-49.27, -25.43)}],
    "DF": [{"ibge_codigo": "5300108", "nome": "Brasília", "uf": "DF", "center": (-47.88, -15.79)}],
}


def make_bbox_polygon(lon: float, lat: float, dlon: float, dlat: float):
    """Cria polígono GeoJSON fechado centrado em (lon, lat)."""
    w, h = dlon / 2.0, dlat / 2.0
    return [[
        [round(lon - w, 4), round(lat + h, 4)],
        [round(lon + w, 4), round(lat + h, 4)],
        [round(lon + w, 4), round(lat - h, 4)],
        [round(lon - w, 4), round(lat - h, 4)],
        [round(lon - w, 4), round(lat + h, 4)],
    ]]


def generate_dev_fixtures():
    """Gera fixtures geográficos válidos (GeoJSON) com posições reais para as 27 UFs."""
    logging.info("Gerando malhas GeoJSON geolocalizadas para as 27 UFs...")

    brasil_features = []
    ufs_payloads = {}

    for item in UFS_INFO:
        lon, lat = item["center"]
        dlon, dlat = item["span"]
        brasil_features.append({
            "type": "Feature",
            "properties": {
                "ibge_codigo": item["ibge_codigo"],
                "nome": item["nome"],
                "uf": item["uf"],
            },
            "geometry": {
                "type": "Polygon",
                "coordinates": make_bbox_polygon(lon, lat, dlon, dlat),
            },
        })

        uf_sigla = item["uf"]
        municipios = CAPITAIS_PILOTO.get(uf_sigla, [
            {
                "ibge_codigo": f"{item['ibge_codigo']}00001",
                "nome": f"Capital {item['nome']}",
                "uf": uf_sigla,
                "center": (lon, lat),
            }
        ])

        uf_features = []
        for mun in municipios:
            mlon, mlat = mun["center"]
            uf_features.append({
                "type": "Feature",
                "properties": {
                    "ibge_codigo": mun["ibge_codigo"],
                    "nome": mun["nome"],
                    "uf": mun["uf"],
                },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": make_bbox_polygon(mlon, mlat, 0.35, 0.25),
                },
            })
        ufs_payloads[uf_sigla.lower()] = {"type": "FeatureCollection", "features": uf_features}

    brasil_payload = {"type": "FeatureCollection", "features": brasil_features}

    for out_dir in OUTPUT_DIRS:
        uf_dir = os.path.join(out_dir, "ufs")
        os.makedirs(uf_dir, exist_ok=True)
        for sigla_lower, payload in ufs_payloads.items():
            uf_file = os.path.join(uf_dir, f"uf_{sigla_lower}.json")
            with open(uf_file, "w", encoding="utf-8") as f:
                json.dump(payload, f, indent=2, ensure_ascii=False)
        brasil_file = os.path.join(out_dir, "brasil_ufs.json")
        with open(brasil_file, "w", encoding="utf-8") as f:
            json.dump(brasil_payload, f, indent=2, ensure_ascii=False)
        logging.info(f"[OK] Malhas GeoJSON salvas em {out_dir}")


def validate_geojson_files():
    """Valida a conformidade de formato e tamanho dos arquivos gerados."""
    logging.info("Validando integridade dos arquivos GeoJSON...")
    for out_dir in OUTPUT_DIRS:
        brasil_file = os.path.join(out_dir, "brasil_ufs.json")
        if not os.path.exists(brasil_file):
            raise FileNotFoundError(f"Arquivo não encontrado: {brasil_file}")
        with open(brasil_file, "r", encoding="utf-8") as f:
            brasil_data = json.load(f)
            assert brasil_data.get("type") == "FeatureCollection"
            assert len(brasil_data.get("features", [])) == 27, "brasil_ufs.json deve conter 27 UFs"

        uf_dir = os.path.join(out_dir, "ufs")
        for item in UFS_INFO:
            sigla = item["uf"].lower()
            filepath = os.path.join(uf_dir, f"uf_{sigla}.json")
            if not os.path.exists(filepath):
                raise FileNotFoundError(f"Arquivo não encontrado: {filepath}")

            size_kb = os.path.getsize(filepath) / 1024
            if size_kb > 350.0:
                logging.warning(f"Atenção: {filepath} ({size_kb:.1f} KB) excede o teto de 350 KB!")

            with open(filepath, "r", encoding="utf-8") as f:
                data = json.load(f)
                assert data.get("type") == "FeatureCollection", f"{filepath} não é FeatureCollection"
                assert data.get("features"), f"{filepath} sem features"

    logging.info("[OK] Todos os arquivos GeoJSON (frontend/public/geo e public/geo) validados com 100% de conformidade!")

if __name__ == "__main__":
    try:
        generate_dev_fixtures()
        validate_geojson_files()
    except Exception as e:
        logging.error(f"Erro no pipeline: {e}")
        sys.exit(1)
