"use client";

import React, { useState, useRef } from "react";
import { Play, Pause, Volume2, FastForward, Download, FileText, Video, Radio, ExternalLink } from "lucide-react";

interface PlayerMultimidiaProps {
  formato: "TEXTO" | "VIDEO" | "AUDIO" | "IMPRESSO_PDF";
  videoUrl?: string;
  audioUrl?: string;
  pdfUrl?: string;
  titulo: string;
}

export const PlayerMultimidia: React.FC<PlayerMultimidiaProps> = ({
  formato,
  videoUrl,
  audioUrl,
  pdfUrl,
  titulo,
}) => {
  // Estados para o Player de Áudio
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [tocando, setTocando] = useState(false);
  const [velocidade, setVelocidade] = useState(1);

  if (formato === "TEXTO") return null;

  // 1. SUPORTE A VÍDEO
  if (formato === "VIDEO" && videoUrl) {
    return (
      <div className="rounded-2xl overflow-hidden shadow-lg border border-slate-200 bg-black aspect-video relative my-6">
        <video
          controls
          playsInline
          className="w-full h-full object-contain"
          src={videoUrl}
        >
          Seu navegador não suporta a reprodução de vídeos HTML5.
        </video>
      </div>
    );
  }

  // 2. SUPORTE A ÁUDIO / PODCAST / PROGRAMA DE RÁDIO REGIONAL
  if (formato === "AUDIO" && audioUrl) {
    const alternarReproducao = () => {
      if (audioRef.current) {
        if (tocando) {
          audioRef.current.pause();
        } else {
          audioRef.current.play();
        }
        setTocando(!tocando);
      }
    };

    const alternarVelocidade = () => {
      if (audioRef.current) {
        const velocidades = [1, 1.25, 1.5, 2];
        const proxima = velocidades[(velocidades.indexOf(velocidade) + 1) % velocidades.length];
        audioRef.current.playbackRate = proxima;
        setVelocidade(proxima);
      }
    };

    return (
      <div className="bg-gradient-to-r from-slate-900 to-blue-950 text-white p-5 rounded-2xl shadow-md border border-slate-700 my-6 space-y-3">
        <audio
          ref={audioRef}
          src={audioUrl}
          onEnded={() => setTocando(false)}
        />

        <div className="flex items-center justify-between text-xs text-blue-300">
          <span className="flex items-center gap-1.5 font-bold uppercase tracking-wider">
            <Radio className="w-4 h-4 text-emerald-400 animate-pulse" />
            Boletim de Áudio / Programa de Rádio
          </span>
          <a
            href={audioUrl}
            download
            className="flex items-center gap-1 hover:text-white transition-colors text-[11px]"
          >
            <Download className="w-3.5 h-3.5" />
            Baixar Áudio (MP3)
          </a>
        </div>

        <div className="flex items-center gap-4">
          {/* Botão Play/Pause */}
          <button
            onClick={alternarReproducao}
            className="w-12 h-12 rounded-full bg-destaque hover:bg-blue-600 text-white flex items-center justify-center shadow-lg transition-transform active:scale-95 flex-shrink-0"
          >
            {tocando ? <Pause className="w-5 h-5 fill-white" /> : <Play className="w-5 h-5 fill-white ml-0.5" />}
          </button>

          {/* Barra de Título / Informação */}
          <div className="flex-1 min-w-0">
            <p className="text-sm font-semibold truncate">{titulo}</p>
            <p className="text-xs text-slate-400">Edição sonora oficial — Ouça a qualquer momento</p>
          </div>

          {/* Botão de Velocidade */}
          <button
            onClick={alternarVelocidade}
            className="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-xs font-bold text-blue-300 border border-slate-700 transition-colors"
          >
            {velocidade}x
          </button>
        </div>
      </div>
    );
  }

  // 3. SUPORTE A PAPER NEWS (INFORMATIVO IMPRESSO EM PDF / TABLOIDE DIGITALIZADO)
  if (formato === "IMPRESSO_PDF" && pdfUrl) {
    return (
      <div className="bg-amber-50 border-2 border-amber-300 rounded-2xl p-6 shadow-sm my-6 space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-10 h-10 rounded-xl bg-amber-200 text-amber-900 flex items-center justify-center font-bold">
              <FileText className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-amber-950 uppercase tracking-wide">
                Edição Impressa Oficial (Paper News)
              </h3>
              <p className="text-xs text-amber-800">
                Informativo diagramado para impressão e distribuição local em papel.
              </p>
            </div>
          </div>

          <span className="hidden sm:inline-block bg-amber-200 text-amber-900 text-xs font-bold px-2.5 py-1 rounded-full uppercase">
            Formato Tabloide
          </span>
        </div>

        {/* Pré-visualizador embutido e ações de download */}
        <div className="flex flex-col sm:flex-row items-center gap-3 pt-2">
          <a
            href={pdfUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="w-full sm:w-auto flex-1 flex items-center justify-center gap-2 bg-slate-900 hover:bg-slate-800 text-white font-semibold px-4 py-2.5 rounded-xl text-xs shadow-sm transition-all"
          >
            <ExternalLink className="w-4 h-4 text-amber-300" />
            Abrir e Folhear Edição em PDF
          </a>

          <a
            href={pdfUrl}
            download
            className="w-full sm:w-auto flex items-center justify-center gap-2 bg-amber-600 hover:bg-amber-700 text-white font-semibold px-4 py-2.5 rounded-xl text-xs shadow-sm transition-all"
          >
            <Download className="w-4 h-4" />
            Baixar para Impressão Local
          </a>
        </div>
      </div>
    );
  }

  return null;
};
