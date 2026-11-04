import React from "react";
import { NoticiaItem } from "@/components/CardNoticia";
import { PortalInterativoLeitor } from "@/components/PortalInterativoLeitor";

async function fetchFeedData(): Promise<{
  modo_eleitoral_ativo: boolean;
  noticias: NoticiaItem[];
}> {
  const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
  try {
    const res = await fetch(`${apiUrl}/api/v1/feed?limit=24`, {
      next: { revalidate: 30 },
    });
    if (res.ok) {
      const data = await res.json();
      if (data.noticias && data.noticias.length > 0) {
        return data;
      }
    }
  } catch (err) {
    // Retorno de contingência em caso de backend offline
  }

  return {
    modo_eleitoral_ativo: false,
    noticias: [
      {
        id: "demo-1",
        titulo: "Convenção Partidária Nacional Homologa Diretrizes do Plano Diretor",
        slug: "convencao-nacional-plano-diretor",
        subtitulo: "Lideranças de todas as 27 Unidades da Federação alinham governança e tecnologia.",
        escopo: "NACIONAL",
        formato: "TEXTO",
        conteudo_sanitizado:
          "<p>Durante o encontro nacional que reuniu representantes das 27 Unidades da Federação, a Executiva ratificou as salvaguardas de conformidade eleitoral e transparência algorítmica.</p>",
        publicado_em: new Date().toISOString(),
        hype_score: 42.8,
      },
      {
        id: "demo-2",
        titulo: "Diretório Estadual de São Paulo Lança Programa de Formação Cidadã",
        slug: "sp-programa-formacao-cidada",
        subtitulo: "Encontros regionais percorrerão o interior paulista no próximo trimestre com transmissão ao vivo.",
        escopo: "ESTADUAL",
        ibge_codigo: "35",
        uf: "SP",
        territorio_nome: "São Paulo (SP)",
        formato: "VIDEO",
        video_url: "https://cdn.jornalfederado.org.br/videos/formacao-sp.mp4",
        conteudo_sanitizado:
          "<p>Acompanhe a cobertura completa em vídeo do lançamento estadual em São Paulo.</p>",
        publicado_em: new Date(Date.now() - 3600000 * 3).toISOString(),
        hype_score: 28.5,
      },
      {
        id: "demo-3",
        titulo: "Boletim de Rádio: Câmara de Belo Horizonte Debate Nova Malha Viária",
        slug: "bh-nova-malha-viaria",
        subtitulo: "Ouça o resumo de 3 minutos produzido pelo núcleo de comunicação da capital mineira.",
        escopo: "MUNICIPAL",
        ibge_codigo: "3106200",
        uf: "MG",
        territorio_nome: "Belo Horizonte - MG",
        formato: "AUDIO",
        audio_url: "https://cdn.jornalfederado.org.br/audios/boletim-bh.mp3",
        conteudo_sanitizado:
          "<p>Proposta foi formulada com participação direta dos núcleos comunitários de Belo Horizonte.</p>",
        publicado_em: new Date(Date.now() - 3600000 * 6).toISOString(),
        hype_score: 19.4,
      },
      {
        id: "demo-4",
        titulo: "Edição Impressa nº 14 — Tabloide Oficial do Diretório de Curitiba",
        slug: "tabloide-curitiba-edicao-14",
        subtitulo: "Baixe o PDF diagramado em 8 páginas pronto para impressão e distribuição nos bairros.",
        escopo: "MUNICIPAL",
        ibge_codigo: "4106900",
        uf: "PR",
        territorio_nome: "Curitiba - PR",
        formato: "IMPRESSO_PDF",
        pdf_edicao_url: "https://cdn.jornalfederado.org.br/jornais/curitiba-ed14.pdf",
        conteudo_sanitizado:
          "<p>Edição especial impressa com o balanço das ações municipais em Curitiba e Região Metropolitana.</p>",
        publicado_em: new Date(Date.now() - 3600000 * 10).toISOString(),
        hype_score: 15.2,
      },
    ],
  };
}

export default async function HomePage() {
  const { modo_eleitoral_ativo, noticias } = await fetchFeedData();

  return (
    <div className="max-w-7xl mx-auto px-4 py-6 md:py-8">
      <PortalInterativoLeitor
        noticiasIniciais={noticias}
        modoEleitoralAtivo={modo_eleitoral_ativo}
      />
    </div>
  );
}
