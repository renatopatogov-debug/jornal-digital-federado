import React from "react";
import { Flame, Clock, MapPin, Video, Radio, FileText, BookOpen } from "lucide-react";

export interface NoticiaItem {
  id: string;
  titulo: string;
  slug: string;
  subtitulo?: string;
  escopo: "NACIONAL" | "ESTADUAL" | "MUNICIPAL";
  ibge_codigo?: string;
  uf?: string;
  territorio_nome?: string;
  formato?: "TEXTO" | "VIDEO" | "AUDIO" | "IMPRESSO_PDF";
  video_url?: string;
  audio_url?: string;
  pdf_edicao_url?: string;
  conteudo_sanitizado: string;
  imagem_destaque_url?: string;
  publicado_em?: string;
  votos_positivos?: number;
  votos_negativos?: number;
  hype_score?: number | null;
}

const NOMES_TERRITORIOS: Record<string, string> = {
  "11": "Rondônia (RO)",
  "12": "Acre (AC)",
  "13": "Amazonas (AM)",
  "15": "Pará (PA)",
  "21": "Maranhão (MA)",
  "23": "Ceará (CE)",
  "26": "Pernambuco (PE)",
  "29": "Bahia (BA)",
  "31": "Minas Gerais (MG)",
  "32": "Espírito Santo (ES)",
  "33": "Rio de Janeiro (RJ)",
  "35": "São Paulo (SP)",
  "41": "Paraná (PR)",
  "42": "Santa Catarina (SC)",
  "43": "Rio Grande do Sul (RS)",
  "50": "Mato Grosso do Sul (MS)",
  "51": "Mato Grosso (MT)",
  "52": "Goiás (GO)",
  "53": "Distrito Federal (DF)",
  "3550308": "São Paulo - SP",
  "3304557": "Rio de Janeiro - RJ",
  "3106200": "Belo Horizonte - MG",
  "4106900": "Curitiba - PR",
  "5300108": "Brasília - DF",
};

interface CardNoticiaProps {
  noticia: NoticiaItem;
  modoEleitoral: boolean;
}

export const CardNoticia: React.FC<CardNoticiaProps> = ({ noticia, modoEleitoral }) => {
  const dataFormatada = noticia.publicado_em
    ? new Date(noticia.publicado_em).toLocaleDateString("pt-BR", {
        day: "2-digit",
        month: "short",
        hour: "2-digit",
        minute: "2-digit",
      })
    : "Recente";

  // Estimativa de tempo de leitura (~180 palavras por minuto)
  const textoLimpo = (noticia.conteudo_sanitizado || "").replace(/<[^>]+>/g, " ");
  const totalPalavras = textoLimpo.trim().split(/\s+/).filter(Boolean).length;
  const minutosLeitura = Math.max(1, Math.ceil(totalPalavras / 180));

  const labelTerritorio =
    noticia.territorio_nome ||
    (noticia.ibge_codigo && NOMES_TERRITORIOS[noticia.ibge_codigo]) ||
    (noticia.escopo === "NACIONAL"
      ? "Nacional • Brasil"
      : `${noticia.escopo} (${noticia.ibge_codigo || ""})`);

  const renderBadgeFormato = () => {
    if (noticia.formato === "VIDEO") {
      return (
        <span className="bg-red-600 text-white text-[10px] font-bold px-2 py-0.5 rounded-md flex items-center gap-1 uppercase tracking-wide shadow-sm">
          <Video className="w-3 h-3" /> Vídeo
        </span>
      );
    }
    if (noticia.formato === "AUDIO") {
      return (
        <span className="bg-purple-600 text-white text-[10px] font-bold px-2 py-0.5 rounded-md flex items-center gap-1 uppercase tracking-wide shadow-sm">
          <Radio className="w-3 h-3" /> Áudio
        </span>
      );
    }
    if (noticia.formato === "IMPRESSO_PDF") {
      return (
        <span className="bg-amber-600 text-white text-[10px] font-bold px-2 py-0.5 rounded-md flex items-center gap-1 uppercase tracking-wide shadow-sm">
          <FileText className="w-3 h-3" /> Paper News
        </span>
      );
    }
    return null;
  };

  return (
    <article className="bg-white rounded-2xl shadow-sm border border-slate-200 overflow-hidden hover:shadow-md hover:border-blue-200 transition-all flex flex-col justify-between group">
      {noticia.imagem_destaque_url && (
        <a href={`/noticia/${noticia.slug}`} className="relative h-48 w-full bg-slate-100 overflow-hidden block">
          <img
            src={noticia.imagem_destaque_url}
            alt={noticia.titulo}
            className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
            loading="lazy"
          />
          <div className="absolute top-3 left-3">{renderBadgeFormato()}</div>
        </a>
      )}

      <div className="p-5 flex-1 flex flex-col">
        {/* Metadados, Território e Formato */}
        <div className="flex items-center justify-between gap-2 text-xs text-slate-500 mb-2.5 flex-wrap">
          <div className="flex items-center gap-2">
            <span className="flex items-center gap-1 font-bold text-destaque uppercase tracking-wide bg-blue-50 px-2 py-0.5 rounded-md border border-blue-100">
              <MapPin className="w-3 h-3" />
              {labelTerritorio}
            </span>
            {!noticia.imagem_destaque_url && renderBadgeFormato()}
          </div>

          <span className="flex items-center gap-1 text-[11px] text-slate-400">
            <Clock className="w-3 h-3" />
            {dataFormatada}
          </span>
        </div>

        {/* Título e Subtítulo */}
        <h2 className="text-lg md:text-xl font-bold text-slate-900 leading-snug group-hover:text-destaque transition-colors mb-2">
          <a href={`/noticia/${noticia.slug}`}>{noticia.titulo}</a>
        </h2>

        {noticia.subtitulo && (
          <p className="text-sm text-slate-600 line-clamp-2 mb-4 leading-relaxed">
            {noticia.subtitulo}
          </p>
        )}

        <div className="mt-auto pt-4 border-t border-slate-100 flex items-center justify-between gap-2">
          <div className="flex items-center gap-3">
            <a
              href={`/noticia/${noticia.slug}`}
              className="text-xs font-bold text-destaque hover:underline"
            >
              Ler pauta &rarr;
            </a>
            <span className="text-[11px] text-slate-400 flex items-center gap-1">
              <BookOpen className="w-3 h-3" />
              {minutosLeitura} min
            </span>
          </div>

          {/* Placar Hype (Visível apenas fora do Modo Eleitoral) */}
          {!modoEleitoral && noticia.hype_score !== null && noticia.hype_score !== undefined && (
            <div
              className="flex items-center gap-1 text-xs font-bold bg-amber-50 text-amber-800 px-2.5 py-1 rounded-full border border-amber-200"
              title="Pontuação de destaque orgânico auditável"
            >
              <Flame className="w-3.5 h-3.5 text-amber-500 fill-amber-500" />
              <span>{noticia.hype_score.toFixed(1)} Hype</span>
            </div>
          )}

          {modoEleitoral && (
            <span className="text-[11px] font-medium text-slate-400 italic">
              Ordem cronológica
            </span>
          )}
        </div>
      </div>
    </article>
  );
};
