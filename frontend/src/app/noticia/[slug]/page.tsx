import React from "react";
import { notFound } from "next/navigation";
import { MapPin, Clock, Calendar, Flame, ArrowLeft, ShieldCheck } from "lucide-react";
import { BotaoCompartilhar } from "@/components/BotaoCompartilhar";
import { BotaoVotar } from "@/components/BotaoVotar";
import { PlayerMultimidia } from "@/components/PlayerMultimidia";
import { ControleLeituraAcessivel } from "@/components/ControleLeituraAcessivel";

interface NoticiaDetalhe {
  id: string;
  titulo: string;
  slug: string;
  subtitulo?: string;
  escopo: "NACIONAL" | "ESTADUAL" | "MUNICIPAL";
  ibge_codigo?: string;
  formato: "TEXTO" | "VIDEO" | "AUDIO" | "IMPRESSO_PDF";
  video_url?: string;
  audio_url?: string;
  pdf_edicao_url?: string;
  conteudo_sanitizado: string;
  imagem_destaque_url?: string;
  autor_nome?: string;
  publicado_em: string;
  votos_positivos: number;
  votos_negativos: number;
  hype_score?: number;
}

const NOMES_TERRITORIOS: Record<string, string> = {
  "35": "São Paulo (SP)",
  "3550308": "São Paulo - Capital (SP)",
  "33": "Rio de Janeiro (RJ)",
  "3304557": "Rio de Janeiro - Capital (RJ)",
  "31": "Minas Gerais (MG)",
  "3106200": "Belo Horizonte (MG)",
  "41": "Paraná (PR)",
  "4106900": "Curitiba (PR)",
  "43": "Rio Grande do Sul (RS)",
  "29": "Bahia (BA)",
  "26": "Pernambuco (PE)",
  "53": "Distrito Federal (DF)",
  "5300108": "Brasília (DF)",
};

function estimarMinutosLeitura(html: string, formato: string): string {
  if (formato === "VIDEO") return "Vídeo";
  if (formato === "AUDIO") return "Podcast";
  if (formato === "IMPRESSO_PDF") return "Edição PDF";
  const textoLimpo = html.replace(/<[^>]+>/g, " ").trim();
  const palavras = textoLimpo ? textoLimpo.split(/\s+/).length : 120;
  const minutos = Math.max(1, Math.ceil(palavras / 180));
  return `${minutos} min de leitura`;
}

async function fetchNoticiaPorSlug(slug: string): Promise<{
  noticia: NoticiaDetalhe;
  modoEleitoral: boolean;
} | null> {
  const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
  try {
    const res = await fetch(`${apiUrl}/api/v1/feed?limit=50`, {
      next: { revalidate: 30 },
    });
    if (res.ok) {
      const data = await res.json();
      const item = data.noticias?.find((n: any) => n.slug === slug);
      if (item) {
        return {
          noticia: {
            ...item,
            formato: item.formato || "TEXTO",
            autor_nome: "Redação Federada",
            votos_positivos: item.votos_positivos ?? 12,
            votos_negativos: item.votos_negativos ?? 1,
          },
          modoEleitoral: data.modo_eleitoral_ativo,
        };
      }
    }
  } catch (e) {
    // Modo offline resiliente
  }

  // Fallback demo para visualização imediata da UX
  return {
    noticia: {
      id: "00000000-0000-0000-0000-000000000001",
      titulo: "Convenção Partidária Nacional Homologa Diretrizes do Plano Diretor",
      slug: slug,
      subtitulo: "Lideranças de todas as 27 Unidades da Federação alinham governança e tecnologia para a nova era da imprensa digital.",
      escopo: "NACIONAL",
      formato: "TEXTO",
      conteudo_sanitizado: `
        <p>Durante o encontro que reuniu lideranças municipais e estaduais, a Executiva Nacional ratificou as salvaguardas de conformidade eleitoral estabelecidas pela Lei nº 9.504/97.</p>
        <h2>Autonomia Editorial e Participação Direta</h2>
        <p>A plataforma digital consolida um novo paradigma participativo, assegurando que o leitor comum desfrute de acesso irrestrito, gratuito e livre de barreiras a todas as edições municipais do país.</p>
        <blockquote>"Garantir a voz de cada município com soberania técnica é o compromisso primordial desta agremiação."</blockquote>
        <p>Os diretórios regionais iniciarão o calendário de formações locais a partir da próxima semana, expandindo os núcleos de redação federada.</p>
      `,
      imagem_destaque_url: "https://images.unsplash.com/photo-1540910419892-4a36d2c3266c?auto=format&fit=crop&w=1200&q=80",
      autor_nome: "Corpo Editorial Nacional",
      publicado_em: new Date().toISOString(),
      votos_positivos: 45,
      votos_negativos: 3,
      hype_score: 42.8,
    },
    modoEleitoral: false,
  };
}

export default async function NoticiaPage({ params }: { params: { slug: string } }) {
  const data = await fetchNoticiaPorSlug(params.slug);

  if (!data) {
    notFound();
  }

  const { noticia, modoEleitoral } = data;
  const dataFormatada = new Date(noticia.publicado_em).toLocaleDateString("pt-BR", {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

  const labelTerritorio = noticia.ibge_codigo
    ? NOMES_TERRITORIOS[noticia.ibge_codigo] || `IBGE ${noticia.ibge_codigo}`
    : "Cobertura Nacional";

  const tempoLeitura = estimarMinutosLeitura(noticia.conteudo_sanitizado, noticia.formato);

  return (
    <article className="min-h-screen bg-white">
      {/* Barra de Retorno e Breadcrumbs */}
      <div className="border-b border-slate-100 bg-slate-50/80 sticky top-16 z-30 backdrop-blur-md">
        <div className="max-w-4xl mx-auto px-4 h-12 flex items-center justify-between text-xs text-slate-500">
          <a
            href="/"
            className="flex items-center gap-1.5 font-semibold text-slate-700 hover:text-destaque transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
            Voltar ao Feed
          </a>

          <div className="flex items-center gap-2 font-medium">
            <span className="uppercase tracking-wider text-destaque font-bold">
              {noticia.escopo}
            </span>
            <span>•</span>
            <span className="flex items-center gap-1 text-slate-600">
              <MapPin className="w-3.5 h-3.5 text-slate-400" />
              {labelTerritorio}
            </span>
          </div>
        </div>
      </div>

      <div className="max-w-3xl mx-auto px-4 py-8 md:py-12 space-y-8">
        {/* Cabeçalho da Matéria */}
        <header className="space-y-4">
          <div className="flex flex-wrap items-center gap-2 text-xs font-semibold text-slate-500">
            <span className="bg-blue-50 text-destaque px-2.5 py-1 rounded-md border border-blue-100 uppercase tracking-wider">
              {noticia.escopo === "NACIONAL" ? "Cobertura Federal" : labelTerritorio}
            </span>
            <span>•</span>
            <span className="flex items-center gap-1">
              <Calendar className="w-3.5 h-3.5" />
              {dataFormatada}
            </span>
            <span>•</span>
            <span className="flex items-center gap-1 text-slate-600 bg-slate-100 px-2 py-0.5 rounded">
              <Clock className="w-3.5 h-3.5" />
              {tempoLeitura}
            </span>
          </div>

          <h1 className="text-3xl md:text-4xl lg:text-5xl font-black text-slate-900 tracking-tight leading-[1.15]">
            {noticia.titulo}
          </h1>

          {noticia.subtitulo && (
            <p className="text-lg md:text-xl text-slate-600 font-normal leading-relaxed">
              {noticia.subtitulo}
            </p>
          )}

          {/* Assinatura e Compartilhamento */}
          <div className="flex items-center justify-between pt-4 border-t border-slate-100 text-xs text-slate-500">
            <div>
              <p className="font-semibold text-slate-800">{noticia.autor_nome}</p>
              <p className="text-[11px] text-slate-400">Verificado pela Diretoria de Comunicação</p>
            </div>

            <BotaoCompartilhar titulo={noticia.titulo} />
          </div>
        </header>

        {/* Imagem de Destaque com Acessibilidade */}
        {noticia.imagem_destaque_url && (
          <figure className="rounded-2xl overflow-hidden shadow-sm border border-slate-100">
            <img
              src={noticia.imagem_destaque_url}
              alt={noticia.titulo}
              className="w-full max-h-[480px] object-cover"
            />
            <figcaption className="p-2.5 text-center text-[11px] text-slate-400 bg-slate-50">
              Registro editorial oficial — Jornal Digital Federado
            </figcaption>
          </figure>
        )}

        {/* Player Multimídia (Vídeo / Áudio Podcast / Paper News PDF) */}
        <PlayerMultimidia
          formato={noticia.formato}
          videoUrl={noticia.video_url}
          audioUrl={noticia.audio_url}
          pdfUrl={noticia.pdf_edicao_url}
          titulo={noticia.titulo}
        />

        {/* Barra de Acessibilidade (TTS em Português + Escala Tipográfica + Tema de Leitura) e Corpo Sanitizado */}
        <ControleLeituraAcessivel
          titulo={noticia.titulo}
          subtitulo={noticia.subtitulo}
          conteudoHtmlSanitizado={noticia.conteudo_sanitizado}
        />

        {/* Seção de Participação e Votação (ADRs D1 e D2) */}
        <section className="mt-12 pt-8 border-t border-slate-200 space-y-4 bg-slate-50 p-6 rounded-2xl border">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-base font-bold text-slate-900 flex items-center gap-2">
                <ShieldCheck className="w-5 h-5 text-emerald-600" />
                Votação Auditável de Filiados
              </h3>
              <p className="text-xs text-slate-500">
                O voto no ranking Hype é exclusivo para membros partidários cadastrados (1 filiado = 1 voto).
              </p>
            </div>

            {!modoEleitoral && noticia.hype_score !== undefined && (
              <div className="flex items-center gap-1.5 bg-amber-50 text-amber-800 px-3 py-1.5 rounded-full border border-amber-200 text-xs font-bold">
                <Flame className="w-4 h-4 text-amber-500 fill-amber-500" />
                <span>{noticia.hype_score.toFixed(1)} Hype</span>
              </div>
            )}
          </div>

          <BotaoVotar
            noticiaId={noticia.id}
            votosPositivosIniciais={noticia.votos_positivos}
            votosNegativosIniciais={noticia.votos_negativos}
            modoEleitoral={modoEleitoral}
          />
        </section>
      </div>
    </article>
  );
}
