"use client";

import React, { useState, useMemo, useEffect, useCallback } from "react";
import {
  PenTool,
  Send,
  ShieldCheck,
  CheckCircle2,
  Eye,
  FileText,
  Bold,
  Italic,
  Heading2,
  Quote,
  List,
  Link as LinkIcon,
  ArrowUpRight,
  Users,
  AlertOctagon,
  Sparkles,
  Save,
  MapPin,
  Clock,
  Flame,
  Layers,
  LogOut,
  Scale,
  KeyRound,
  History,
  RefreshCw,
  XCircle,
  Laptop,
  Lock,
} from "lucide-react";
import { CardNoticia } from "@/components/CardNoticia";
import { PlayerMultimidia } from "@/components/PlayerMultimidia";
import { buscarSessaoAtual, encerrarSessao, SessaoUsuario } from "@/lib/auth";

const CATALOGO_ESTADOS_IBGE = [
  { ibge: "35", nome: "São Paulo (SP)" },
  { ibge: "33", nome: "Rio de Janeiro (RJ)" },
  { ibge: "31", nome: "Minas Gerais (MG)" },
  { ibge: "41", nome: "Paraná (PR)" },
  { ibge: "43", nome: "Rio Grande do Sul (RS)" },
  { ibge: "29", nome: "Bahia (BA)" },
  { ibge: "26", nome: "Pernambuco (PE)" },
  { ibge: "23", nome: "Ceará (CE)" },
  { ibge: "15", nome: "Pará (PA)" },
  { ibge: "52", nome: "Goiás (GO)" },
  { ibge: "53", nome: "Distrito Federal (DF)" },
];

const CATALOGO_MUNICIPIOS_IBGE = [
  { ibge: "3550308", nome: "São Paulo - Capital (SP)" },
  { ibge: "3509502", nome: "Campinas (SP)" },
  { ibge: "3304557", nome: "Rio de Janeiro - Capital (RJ)" },
  { ibge: "3106200", nome: "Belo Horizonte (MG)" },
  { ibge: "4106900", nome: "Curitiba (PR)" },
  { ibge: "4314902", nome: "Porto Alegre (RS)" },
  { ibge: "2927408", nome: "Salvador (BA)" },
  { ibge: "2611606", nome: "Recife (PE)" },
  { ibge: "5300108", nome: "Brasília (DF)" },
  { ibge: "OUTRO", nome: "Digitar outro código IBGE municipal (7 dígitos)..." },
];

interface MateriaAcervo {
  id: string;
  titulo: string;
  slug: string;
  escopo: "MUNICIPAL" | "ESTADUAL" | "NACIONAL";
  ibge_codigo?: string | null;
  formato: "TEXTO" | "VIDEO" | "AUDIO" | "IMPRESSO_PDF";
  status: "PUBLICADO" | "RASCUNHO" | "EM_REVISAO" | "ARQUIVADO" | "REJEITADO";
  hype_score: number;
  publicado_em?: string | null;
  escalonamento_solicitado?: "MUNICIPAL" | "ESTADUAL" | "NACIONAL" | null;
}

interface SolicitacaoEscalonamentoItem {
  id: string;
  noticia_id: string;
  noticia_titulo: string;
  noticia_slug: string;
  noticia_ibge_codigo: string | null;
  solicitante_nome: string;
  escopo_origem: "MUNICIPAL" | "ESTADUAL" | "NACIONAL";
  escopo_destino: "MUNICIPAL" | "ESTADUAL" | "NACIONAL";
  status: "PENDENTE" | "APROVADO" | "REJEITADO";
  justificativa: string;
  parecer_revisao: string | null;
  avaliado_por_nome: string | null;
  criado_em: string;
  avaliado_em: string | null;
}

interface ModoEleitoralState {
  ativo: boolean;
  banner_obrigatorio: boolean;
  banner_texto: string;
  freeze_hype: boolean;
  motivo: string;
}

interface SessaoAtivaItem {
  ip_origem: string;
  user_agent: string;
  criado_em: string;
  expira_em: string;
  ultimo_acesso: string;
  sessao_atual: boolean;
}

interface AuditoriaLogItem {
  id: number;
  ator_id: string | null;
  ator_nome: string | null;
  acao: string;
  entidade: string;
  entidade_id: string | null;
  ip_origem: string;
  criado_em: string;
}

export default function CMSPage() {
  const [abaAtiva, setAbaAtiva] = useState<
    "redigir" | "escalonamento" | "governanca" | "seguranca"
  >("redigir");
  const [sessaoAtual, setSessaoAtual] = useState<SessaoUsuario | null>(null);
  const [carregandoSessao, setCarregandoSessao] = useState<boolean>(true);
  const [saindo, setSaindo] = useState<boolean>(false);

  // Estados do Editor Editorial
  const [titulo, setTitulo] = useState("");
  const [subtitulo, setSubtitulo] = useState("");
  const [escopo, setEscopo] = useState<"MUNICIPAL" | "ESTADUAL" | "NACIONAL">("MUNICIPAL");
  const [ibgeSelecionado, setIbgeSelecionado] = useState("3550308");
  const [ibgeCustomizado, setIbgeCustomizado] = useState("");
  const [formato, setFormato] = useState<"TEXTO" | "VIDEO" | "AUDIO" | "IMPRESSO_PDF">("TEXTO");
  const [imagemDestaqueUrl, setImagemDestaqueUrl] = useState("");
  const [videoUrl, setVideoUrl] = useState("");
  const [audioUrl, setAudioUrl] = useState("");
  const [pdfEdicaoUrl, setPdfEdicaoUrl] = useState("");
  const [conteudoHtml, setConteudoHtml] = useState(
    "<p>O diretório reuniu lideranças e filiados para debater as prioridades regionais do próximo semestre.</p>\n<h2>Principais Deliberações</h2>\n<p>Foram aprovadas metas de expansão da comunicação comunitária e formação política continuada.</p>"
  );
  const [modoVisualizacao, setModoVisualizacao] = useState<"editor" | "split" | "preview">("split");
  const [conteudoSanitizadoPreview, setConteudoSanitizadoPreview] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);
  const [mensagemFeedback, setMensagemFeedback] = useState<{
    tipo: "sucesso" | "alerta";
    texto: string;
  } | null>(null);

  // Estados da Aba de Acervo & Escalonamento Federado
  const [acervo, setAcervo] = useState<MateriaAcervo[]>([
    {
      id: "00000000-0000-0000-0000-000000000002",
      titulo: "Diretório Estadual de São Paulo Lança Programa de Formação Política",
      slug: "diretorio-estadual-sp-programa-formacao",
      escopo: "ESTADUAL",
      ibge_codigo: "35",
      formato: "VIDEO",
      status: "PUBLICADO",
      hype_score: 28.4,
      publicado_em: new Date().toISOString(),
    },
    {
      id: "00000000-0000-0000-0000-000000000005",
      titulo: "Mutirão de Cidadania e Ouvidoria Comunitária nos Bairros de São Paulo",
      slug: "mutirao-cidadania-ouvidoria-sao-paulo",
      escopo: "MUNICIPAL",
      ibge_codigo: "3550308",
      formato: "TEXTO",
      status: "PUBLICADO",
      hype_score: 19.2,
      publicado_em: new Date().toISOString(),
    },
  ]);

  // Estados da Aba de Governança & Filiados (ADR D2)
  const [cotaAtivacoesUsada, setCotaAtivacoesUsada] = useState(14);
  const [emailNovoFiliado, setEmailNovoFiliado] = useState("");
  const [cpfHashNovoFiliado, setCpfHashNovoFiliado] = useState("");
  const [liderDissidenteAlvo, setLiderDissidenteAlvo] = useState("");
  const [motivoKillSwitch, setMotivoKillSwitch] = useState("");

  // Estados do Pacote A: Fila de Escalonamento, Modo Eleitoral, 2FA, Sessões e Auditoria
  const [filaEscalonamento, setFilaEscalonamento] = useState<SolicitacaoEscalonamentoItem[]>([]);
  const [parecerPorSolicitacao, setParecerPorSolicitacao] = useState<Record<string, string>>({});
  const [avaliandoId, setAvaliandoId] = useState<string | null>(null);

  const [modoEleitoral, setModoEleitoral] = useState<ModoEleitoralState>({
    ativo: false,
    banner_obrigatorio: true,
    banner_texto:
      "MODO ELEITORAL ATIVO (Lei nº 9.504/97): Ordenação algorítmica Hype suspensa. Exibição estritamente cronológica.",
    freeze_hype: false,
    motivo: "Operação ordinária.",
  });
  const [motivoModoEleitoral, setMotivoModoEleitoral] = useState(
    "Adequação ao calendário eleitoral oficial (Lei nº 9.504/97)."
  );
  const [salvandoModoEleitoral, setSalvandoModoEleitoral] = useState(false);

  const [setup2faDados, setSetup2faDados] = useState<{
    secret_base32: string;
    otpauth_url: string;
  } | null>(null);
  const [codigoConfirmacao2fa, setCodigoConfirmacao2fa] = useState("");
  const [codigosRecuperacaoGerados, setCodigosRecuperacaoGerados] = useState<string[] | null>(
    null
  );
  const [processando2fa, setProcessando2fa] = useState(false);

  const [sessoesAtivas, setSessoesAtivas] = useState<SessaoAtivaItem[]>([]);
  const [logsAuditoria, setLogsAuditoria] = useState<AuditoriaLogItem[]>([]);
  const [visaoGlobalAuditoria, setVisaoGlobalAuditoria] = useState(false);
  const [carregandoPainelSeguranca, setCarregandoPainelSeguranca] = useState(false);

  const carregarDadosOperacionais = useCallback(async () => {
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    setCarregandoPainelSeguranca(true);
    try {
      const [acervoRes, escRes, eleitoralRes, sessoesRes, audRes] = await Promise.allSettled([
        fetch(`${apiUrl}/api/v1/noticias/acervo`, { credentials: "include", cache: "no-store" }),
        fetch(`${apiUrl}/api/v1/escalonamentos`, { credentials: "include", cache: "no-store" }),
        fetch(`${apiUrl}/api/v1/infra/modo-eleitoral`, {
          credentials: "include",
          cache: "no-store",
        }),
        fetch(`${apiUrl}/api/v1/auth/sessoes`, { credentials: "include", cache: "no-store" }),
        fetch(`${apiUrl}/api/v1/infra/auditoria`, { credentials: "include", cache: "no-store" }),
      ]);

      if (acervoRes.status === "fulfilled" && acervoRes.value.ok) {
        const data: {
          noticias?: Array<{
            id: string;
            titulo: string;
            slug: string;
            escopo: "MUNICIPAL" | "ESTADUAL" | "NACIONAL";
            ibge_codigo?: string | null;
            formato: "TEXTO" | "VIDEO" | "AUDIO" | "IMPRESSO_PDF";
            status: "PUBLICADO" | "RASCUNHO" | "EM_REVISAO" | "ARQUIVADO" | "REJEITADO";
            hype_score: number;
            publicado_em?: string | null;
            escalonamento_pendente_destino?: "MUNICIPAL" | "ESTADUAL" | "NACIONAL" | null;
          }>;
        } | null = await acervoRes.value.json().catch(() => null);
        if (data?.noticias) {
          setAcervo(
            data.noticias.map((n) => ({
              id: n.id,
              titulo: n.titulo,
              slug: n.slug,
              escopo: n.escopo,
              ibge_codigo: n.ibge_codigo,
              formato: n.formato,
              status: n.status,
              hype_score: n.hype_score,
              publicado_em: n.publicado_em,
              escalonamento_solicitado: n.escalonamento_pendente_destino ?? null,
            }))
          );
        }
      }

      if (escRes.status === "fulfilled" && escRes.value.ok) {
        const data: { solicitacoes?: SolicitacaoEscalonamentoItem[] } | null = await escRes.value
          .json()
          .catch(() => null);
        if (data?.solicitacoes) {
          setFilaEscalonamento(data.solicitacoes);
        }
      }

      if (eleitoralRes.status === "fulfilled" && eleitoralRes.value.ok) {
        const data: { config?: ModoEleitoralState } | null = await eleitoralRes.value
          .json()
          .catch(() => null);
        if (data?.config) {
          setModoEleitoral(data.config);
        }
      }

      if (sessoesRes.status === "fulfilled" && sessoesRes.value.ok) {
        const data: { sessoes?: SessaoAtivaItem[] } | null = await sessoesRes.value
          .json()
          .catch(() => null);
        if (data?.sessoes) {
          setSessoesAtivas(data.sessoes);
        }
      }

      if (audRes.status === "fulfilled" && audRes.value.ok) {
        const data: { visao_global?: boolean; logs?: AuditoriaLogItem[] } | null =
          await audRes.value.json().catch(() => null);
        if (data?.logs) {
          setLogsAuditoria(data.logs);
          setVisaoGlobalAuditoria(Boolean(data.visao_global));
        }
      }
    } finally {
      setCarregandoPainelSeguranca(false);
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    buscarSessaoAtual(controller.signal).then((sessao) => {
      if (!controller.signal.aborted) {
        setSessaoAtual(sessao);
        setCarregandoSessao(false);
        if (sessao) {
          void carregarDadosOperacionais();
        }
      }
    });
    return () => controller.abort();
  }, [carregarDadosOperacionais]);

  const handleLogoutCms = async () => {
    setSaindo(true);
    await encerrarSessao();
    setSessaoAtual(null);
    window.location.href = "/auth/login";
  };

  useEffect(() => {
    const controller = new AbortController();
    const timer = setTimeout(async () => {
      try {
        const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
        const res = await fetch(`${apiUrl}/api/v1/noticias/preview-sanitizado`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          credentials: "include",
          signal: controller.signal,
          body: JSON.stringify({ conteudo_html: conteudoHtml }),
        });
        if (res.ok) {
          const data: { conteudo_sanitizado?: string } | null = await res.json().catch(() => null);
          setConteudoSanitizadoPreview(data?.conteudo_sanitizado ?? null);
        } else {
          setConteudoSanitizadoPreview(null);
        }
      } catch {
        if (!controller.signal.aborted) {
          setConteudoSanitizadoPreview(null);
        }
      }
    }, 350);

    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [conteudoHtml]);

  // Código IBGE final resolvido conforme o escopo
  const codigoIbgeFinal = useMemo(() => {
    if (escopo === "NACIONAL") return null;
    if (escopo === "ESTADUAL") return ibgeSelecionado.slice(0, 2);
    return ibgeSelecionado === "OUTRO" ? ibgeCustomizado.trim() : ibgeSelecionado;
  }, [escopo, ibgeSelecionado, ibgeCustomizado]);

  // Troca de escopo ajusta código IBGE padrão automaticamente
  const handleEscopoChange = (novoEscopo: "MUNICIPAL" | "ESTADUAL" | "NACIONAL") => {
    setEscopo(novoEscopo);
    if (novoEscopo === "ESTADUAL") {
      setIbgeSelecionado("35");
    } else if (novoEscopo === "MUNICIPAL") {
      setIbgeSelecionado("3550308");
    }
  };

  // Barra de ferramentas Rich-Text assistida
  const inserirBlocoFormatado = (tag: "h2" | "strong" | "em" | "blockquote" | "ul" | "a") => {
    const snippets: Record<typeof tag, string> = {
      h2: "\n<h2>Subtítulo da Seção</h2>\n",
      strong: " <strong>texto em destaque</strong> ",
      em: " <em>texto em itálico</em> ",
      blockquote: '\n<blockquote>"Citação oficial da liderança regional ou trecho em destaque."</blockquote>\n',
      ul: "\n<ul>\n  <li>Primeiro ponto de destaque</li>\n  <li>Segundo ponto de destaque</li>\n</ul>\n",
      a: ' <a href="https://jornalfederado.org.br" rel="noopener noreferrer">link de referência</a> ',
    };
    setConteudoHtml((atual) => `${atual}${snippets[tag]}`);
  };

  // Estatísticas de redação em tempo real
  const estatisticasTexto = useMemo(() => {
    const textoLimpo = conteudoHtml.replace(/<[^>]+>/g, " ").trim();
    const palavras = textoLimpo ? textoLimpo.split(/\s+/).length : 0;
    const minutos = Math.max(1, Math.ceil(palavras / 180));
    return { palavras, minutos };
  }, [conteudoHtml]);

  const submeterMateria = async (publicarAgora: boolean) => {
    setMensagemFeedback(null);
    if (!carregandoSessao && !sessaoAtual) {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Sessão editorial não identificada. Faça login em /auth/admin antes de publicar.",
      });
      return;
    }

    setEnviando(true);
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    const payload = {
      titulo,
      subtitulo: subtitulo || null,
      escopo,
      ibge_codigo: codigoIbgeFinal,
      formato,
      imagem_destaque_url: imagemDestaqueUrl || null,
      video_url: formato === "VIDEO" ? videoUrl : null,
      audio_url: formato === "AUDIO" ? audioUrl : null,
      pdf_edicao_url: formato === "IMPRESSO_PDF" ? pdfEdicaoUrl : null,
      conteudo_html: conteudoHtml,
      publicar_agora: publicarAgora,
    };

    try {
      const res = await fetch(`${apiUrl}/api/v1/noticias`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify(payload),
      });

      if (res.status === 401) {
        setSessaoAtual(null);
      }

      const data: {
        noticia_id?: string;
        id?: string;
        slug?: string;
        mensagem?: string;
      } | null = await res.json().catch(() => null);

      if (!res.ok || !data) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Não foi possível publicar ou salvar a matéria no servidor.",
        });
        return;
      }

      const noticiaId = data.noticia_id || data.id || `srv-${Date.now()}`;
      const slugFinal = data.slug || "materia";

      setAcervo((prev) => [
        {
          id: noticiaId,
          titulo,
          slug: slugFinal,
          escopo,
          ibge_codigo: codigoIbgeFinal || undefined,
          formato,
          status: publicarAgora ? "PUBLICADO" : "RASCUNHO",
          hype_score: 0.0,
          publicado_em: new Date().toISOString(),
        },
        ...prev,
      ]);
      setMensagemFeedback({
        tipo: "sucesso",
        texto:
          data.mensagem ||
          (publicarAgora
            ? `Matéria publicada no Feed (${escopo}) e sanitizada via Ammonia! Slug: /noticia/${slugFinal}`
            : `Rascunho salvo com segurança no acervo editorial (${slugFinal}).`),
      });
      setTitulo("");
      setSubtitulo("");
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Falha de comunicação com a API editorial ao tentar salvar a matéria.",
      });
    } finally {
      setEnviando(false);
    }
  };

  const handleSolicitarEscalonamento = async (id: string, destino: "ESTADUAL" | "NACIONAL") => {
    setMensagemFeedback(null);
    const item = acervo.find((m) => m.id === id);

    if (!carregandoSessao && !sessaoAtual) {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Sessão editorial não identificada. Faça login em /auth/admin para solicitar escalonamento.",
      });
      return;
    }
    if (!item) {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Matéria não encontrada no acervo atual.",
      });
      return;
    }

    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    try {
      const res = await fetch(`${apiUrl}/api/v1/noticias/${id}/escalonar`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          escopo_origem: item.escopo,
          escopo_destino: destino,
          justificativa: "Alto engajamento no Hype regional e relevância federativa.",
        }),
      });

      if (res.status === 401) {
        setSessaoAtual(null);
      }

      const data: { sucesso?: boolean; mensagem?: string } | null = await res
        .json()
        .catch(() => null);

      if (!res.ok || !data?.sucesso) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Não foi possível registrar o pedido de escalonamento.",
        });
        return;
      }

      setAcervo((prev) =>
        prev.map((m) => (m.id === id ? { ...m, escalonamento_solicitado: destino } : m))
      );
      void carregarDadosOperacionais();
      setMensagemFeedback({
        tipo: "sucesso",
        texto:
          data.mensagem ||
          `Solicitação de escalonamento para a vitrine ${destino} enviada à mesa editora superior!`,
      });
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Erro de comunicação com a API ao solicitar escalonamento editorial.",
      });
    }
  };

  const handleAvaliarEscalonamento = async (solicitacaoId: string, aprovado: boolean) => {
    setMensagemFeedback(null);
    setAvaliandoId(solicitacaoId);
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    try {
      const res = await fetch(`${apiUrl}/api/v1/escalonamentos/${solicitacaoId}/avaliar`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          aprovado,
          parecer_revisao: parecerPorSolicitacao[solicitacaoId]?.trim() || null,
        }),
      });
      const data: { sucesso?: boolean; mensagem?: string } | null = await res
        .json()
        .catch(() => null);
      if (!res.ok || !data?.sucesso) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Não foi possível avaliar a solicitação de escalonamento.",
        });
        return;
      }
      setMensagemFeedback({
        tipo: "sucesso",
        texto:
          data.mensagem ||
          (aprovado
            ? "Escalonamento aprovado e escopo da matéria promovido!"
            : "Solicitação de escalonamento rejeitada."),
      });
      await carregarDadosOperacionais();
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Erro de comunicação ao avaliar escalonamento editorial.",
      });
    } finally {
      setAvaliandoId(null);
    }
  };

  const handleAlternarModoEleitoral = async (novoAtivo: boolean) => {
    setMensagemFeedback(null);
    if (motivoModoEleitoral.trim().length < 5) {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Informe uma justificativa jurídica/operacional com pelo menos 5 caracteres.",
      });
      return;
    }
    setSalvandoModoEleitoral(true);
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    try {
      const res = await fetch(`${apiUrl}/api/v1/infra/modo-eleitoral`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          ativo: novoAtivo,
          motivo: motivoModoEleitoral.trim(),
          banner_texto: modoEleitoral.banner_texto,
        }),
      });
      const data: {
        sucesso?: boolean;
        config?: ModoEleitoralState;
        mensagem?: string;
      } | null = await res.json().catch(() => null);
      if (!res.ok || !data?.sucesso || !data.config) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Sem permissão ou falha ao atualizar o Modo Eleitoral.",
        });
        return;
      }
      setModoEleitoral(data.config);
      setMensagemFeedback({
        tipo: novoAtivo ? "alerta" : "sucesso",
        texto: data.mensagem || "Modo Eleitoral atualizado com sucesso.",
      });
      await carregarDadosOperacionais();
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Falha de comunicação ao alternar o Modo Eleitoral.",
      });
    } finally {
      setSalvandoModoEleitoral(false);
    }
  };

  const handleIniciarSetup2fa = async () => {
    setMensagemFeedback(null);
    setProcessando2fa(true);
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    try {
      const res = await fetch(`${apiUrl}/api/v1/auth/2fa/setup`, {
        method: "POST",
        credentials: "include",
      });
      const data: {
        sucesso?: boolean;
        secret_base32?: string;
        otpauth_url?: string;
        mensagem?: string;
      } | null = await res.json().catch(() => null);
      if (!res.ok || !data?.sucesso || !data.secret_base32 || !data.otpauth_url) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Não foi possível iniciar o provisionamento 2FA TOTP.",
        });
        return;
      }
      setSetup2faDados({
        secret_base32: data.secret_base32,
        otpauth_url: data.otpauth_url,
      });
      setCodigosRecuperacaoGerados(null);
      setMensagemFeedback({
        tipo: "sucesso",
        texto: data.mensagem || "Segredo TOTP gerado! Confirme um código de 6 dígitos.",
      });
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Erro de comunicação ao iniciar o setup de 2FA.",
      });
    } finally {
      setProcessando2fa(false);
    }
  };

  const handleConfirmarSetup2fa = async (e: React.FormEvent) => {
    e.preventDefault();
    setMensagemFeedback(null);
    setProcessando2fa(true);
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    try {
      const res = await fetch(`${apiUrl}/api/v1/auth/2fa/confirmar`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ codigo_totp: codigoConfirmacao2fa.trim() }),
      });
      const data: {
        sucesso?: boolean;
        recovery_codes?: string[];
        mensagem?: string;
      } | null = await res.json().catch(() => null);
      if (!res.ok || !data?.sucesso) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Código TOTP inválido.",
        });
        return;
      }
      setCodigosRecuperacaoGerados(data.recovery_codes ?? []);
      setSetup2faDados(null);
      setCodigoConfirmacao2fa("");
      setSessaoAtual((prev) => (prev ? { ...prev, totp_habilitado: true } : prev));
      setMensagemFeedback({
        tipo: "sucesso",
        texto: data.mensagem || "2FA TOTP ativado com sucesso!",
      });
      await carregarDadosOperacionais();
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Erro de comunicação ao confirmar o 2FA.",
      });
    } finally {
      setProcessando2fa(false);
    }
  };

  const handleRevogarOutrasSessoes = async () => {
    setMensagemFeedback(null);
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    try {
      const res = await fetch(`${apiUrl}/api/v1/auth/sessoes/revogar-todas`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ manter_sessao_atual: true }),
      });
      const data: { sucesso?: boolean; mensagem?: string } | null = await res
        .json()
        .catch(() => null);
      if (!res.ok || !data?.sucesso) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Não foi possível revogar as demais sessões.",
        });
        return;
      }
      setMensagemFeedback({
        tipo: "sucesso",
        texto: data.mensagem || "Demais sessões revogadas com sucesso.",
      });
      await carregarDadosOperacionais();
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Erro de comunicação ao revogar sessões.",
      });
    }
  };

  const handleAtivarFiliado = async (e: React.FormEvent) => {
    e.preventDefault();
    setMensagemFeedback(null);
    if (cotaAtivacoesUsada >= 50) return;

    if (!carregandoSessao && !sessaoAtual) {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Faça login como liderança em /auth/admin antes de habilitar filiados.",
      });
      return;
    }

    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    try {
      const res = await fetch(`${apiUrl}/api/v1/usuarios/ativar`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          email: emailNovoFiliado.trim(),
        }),
      });
      if (res.status === 401) {
        setSessaoAtual(null);
      }
      const data: { sucesso?: boolean; mensagem?: string } | null = await res
        .json()
        .catch(() => null);

      if (!res.ok || !data?.sucesso) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Não foi possível habilitar o filiado informado.",
        });
        return;
      }

      setCotaAtivacoesUsada((c) => c + 1);
      setMensagemFeedback({
        tipo: "sucesso",
        texto:
          data.mensagem ||
          `Filiado (${emailNovoFiliado}) habilitado para votação Hype! Cota semanal: ${
            cotaAtivacoesUsada + 1
          }/50.`,
      });
      setEmailNovoFiliado("");
      setCpfHashNovoFiliado("");
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Erro de comunicação com a API ao ativar o filiado.",
      });
    }
  };

  const handleKillSwitchRevogacao = async (e: React.FormEvent) => {
    e.preventDefault();
    setMensagemFeedback(null);
    if (!motivoKillSwitch.trim() || !liderDissidenteAlvo.trim()) return;

    if (!carregandoSessao && !sessaoAtual) {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Faça login com alçada superior em /auth/admin para executar o Kill-Switch.",
      });
      return;
    }

    const alvo = liderDissidenteAlvo.trim();
    const isUuid =
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(alvo);

    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
    try {
      const res = await fetch(`${apiUrl}/api/v1/usuarios/revogar-lote`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          lider_dissidente_id: isUuid ? alvo : null,
          lider_dissidente_email: isUuid ? null : alvo,
          motivo: motivoKillSwitch.trim(),
        }),
      });
      if (res.status === 401) {
        setSessaoAtual(null);
      }

      const data: { sucesso?: boolean; mensagem?: string } | null = await res
        .json()
        .catch(() => null);

      if (!res.ok || !data?.sucesso) {
        setMensagemFeedback({
          tipo: "alerta",
          texto: data?.mensagem || "Falha ao executar revogação em lote.",
        });
        return;
      }

      setMensagemFeedback({
        tipo: "alerta",
        texto: data.mensagem || "Kill-Switch executado com sucesso no servidor.",
      });
      setLiderDissidenteAlvo("");
      setMotivoKillSwitch("");
    } catch {
      setMensagemFeedback({
        tipo: "alerta",
        texto: "Falha de comunicação com a API ao executar o Kill-Switch.",
      });
    }
  };

  return (
    <div className="max-w-7xl mx-auto px-4 py-8 space-y-6">
      {/* Cabeçalho do Estúdio Editorial */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-slate-200 pb-5">
        <div>
          <div className="flex flex-wrap items-center gap-2 mb-2">
            <div className="inline-flex items-center gap-1.5 text-[11px] font-bold uppercase tracking-wider text-destaque bg-blue-50 px-2.5 py-1 rounded-md border border-blue-200">
              <ShieldCheck className="w-3.5 h-3.5" />
              {carregandoSessao
                ? "Verificando sessão HTTP-Only..."
                : sessaoAtual
                ? `${sessaoAtual.nome} • ${sessaoAtual.cargo_nome} (${sessaoAtual.escopo})`
                : "Sessão não autenticada"}
            </div>
            {sessaoAtual ? (
              <button
                type="button"
                onClick={handleLogoutCms}
                disabled={saindo}
                className="inline-flex items-center gap-1 text-[11px] font-bold text-slate-600 hover:text-red-700 bg-slate-100 hover:bg-red-50 px-2.5 py-1 rounded-md border border-slate-200 transition-colors"
              >
                <LogOut className="w-3.5 h-3.5" />
                {saindo ? "Saindo..." : "Sair"}
              </button>
            ) : (
              !carregandoSessao && (
                <a
                  href="/auth/admin"
                  className="inline-flex items-center gap-1 text-[11px] font-bold text-white bg-destaque hover:bg-blue-700 px-2.5 py-1 rounded-md transition-colors"
                >
                  Fazer Login Editorial
                </a>
              )
            )}
          </div>
          <h1 className="text-2xl md:text-3xl font-black text-slate-900 flex items-center gap-2.5">
            <PenTool className="w-7 h-7 text-destaque" />
            Estúdio de Redação & Governança Federada
          </h1>
          <p className="text-sm text-slate-500">
            Produção multimídia assistida, pré-visualização em tempo real, escalonamento federado e gestão de filiados.
          </p>
        </div>

        {/* Navegação por Abas */}
        <div className="flex flex-wrap items-center bg-slate-200/80 p-1 rounded-xl self-start gap-1">
          <button
            type="button"
            onClick={() => setAbaAtiva("redigir")}
            className={`flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-bold transition-all ${
              abaAtiva === "redigir"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            <PenTool className="w-3.5 h-3.5" />
            1. Redigir & Publicar
          </button>
          <button
            type="button"
            onClick={() => setAbaAtiva("escalonamento")}
            className={`flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-bold transition-all ${
              abaAtiva === "escalonamento"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            <Layers className="w-3.5 h-3.5" />
            2. Acervo & Escalonamento ({acervo.length})
          </button>
          <button
            type="button"
            onClick={() => setAbaAtiva("governanca")}
            className={`flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-bold transition-all ${
              abaAtiva === "governanca"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            <Users className="w-3.5 h-3.5" />
            3. Governança & Filiados
          </button>
          <button
            type="button"
            onClick={() => setAbaAtiva("seguranca")}
            className={`flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-bold transition-all ${
              abaAtiva === "seguranca"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            <Lock className="w-3.5 h-3.5" />
            4. Segurança, Eleitoral & Auditoria
          </button>
        </div>
      </div>

      {/* Banner de Feedback */}
      {mensagemFeedback && (
        <div
          className={`p-4 rounded-xl flex items-center justify-between gap-3 text-sm border ${
            mensagemFeedback.tipo === "sucesso"
              ? "bg-emerald-50 border-emerald-300 text-emerald-900"
              : "bg-amber-50 border-amber-300 text-amber-900"
          }`}
        >
          <div className="flex items-center gap-2.5">
            {mensagemFeedback.tipo === "sucesso" ? (
              <CheckCircle2 className="w-5 h-5 text-emerald-600 flex-shrink-0" />
            ) : (
              <AlertOctagon className="w-5 h-5 text-amber-600 flex-shrink-0" />
            )}
            <span className="font-medium">{mensagemFeedback.texto}</span>
          </div>
          <button
            type="button"
            onClick={() => setMensagemFeedback(null)}
            className="text-xs font-bold underline opacity-75 hover:opacity-100"
          >
            Fechar
          </button>
        </div>
      )}

      {/* ABA 1: REDIGIR & PUBLICAR COM PREVIEW AO VIVO */}
      {abaAtiva === "redigir" && (
        <div className="space-y-4">
          {/* Seletor de Modo de Tela (Somente Editor / Lado a Lado / Somente Preview) */}
          <div className="flex items-center justify-between bg-white px-4 py-2.5 rounded-xl border border-slate-200 shadow-sm">
            <div className="flex items-center gap-4 text-xs text-slate-600">
              <span className="flex items-center gap-1.5 font-semibold text-slate-800">
                <FileText className="w-4 h-4 text-destaque" />
                {estatisticasTexto.palavras} palavras
              </span>
              <span>•</span>
              <span className="flex items-center gap-1">
                <Clock className="w-3.5 h-3.5 text-slate-400" />~{estatisticasTexto.minutos} min de leitura
              </span>
            </div>

            <div className="flex items-center gap-1 bg-slate-100 p-1 rounded-lg">
              <button
                type="button"
                onClick={() => setModoVisualizacao("editor")}
                className={`px-2.5 py-1 rounded text-xs font-semibold ${
                  modoVisualizacao === "editor" ? "bg-white text-slate-900 shadow-sm" : "text-slate-600"
                }`}
              >
                Apenas Editor
              </button>
              <button
                type="button"
                onClick={() => setModoVisualizacao("split")}
                className={`px-2.5 py-1 rounded text-xs font-semibold hidden lg:inline-block ${
                  modoVisualizacao === "split" ? "bg-white text-slate-900 shadow-sm" : "text-slate-600"
                }`}
              >
                Lado a Lado (Split)
              </button>
              <button
                type="button"
                onClick={() => setModoVisualizacao("preview")}
                className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-semibold ${
                  modoVisualizacao === "preview" ? "bg-white text-slate-900 shadow-sm" : "text-slate-600"
                }`}
              >
                <Eye className="w-3.5 h-3.5" />
                Pré-visualização do Leitor
              </button>
            </div>
          </div>

          <div
            className={`grid gap-6 ${
              modoVisualizacao === "split" ? "grid-cols-1 lg:grid-cols-12" : "grid-cols-1"
            }`}
          >
            {/* Coluna do Formulário Editorial */}
            {(modoVisualizacao === "editor" || modoVisualizacao === "split") && (
              <div
                className={`${
                  modoVisualizacao === "split" ? "lg:col-span-7" : "max-w-4xl mx-auto w-full"
                } bg-white p-6 rounded-2xl shadow-sm border border-slate-200 space-y-5`}
              >
                <div>
                  <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1.5">
                    Título Principal da Matéria *
                  </label>
                  <input
                    type="text"
                    required
                    value={titulo}
                    onChange={(e) => setTitulo(e.target.value)}
                    placeholder="Ex: Novo Diretório Municipal Inaugura Centro de Formação Cidadã"
                    className="w-full px-4 py-2.5 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 font-semibold"
                  />
                </div>

                <div>
                  <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1.5">
                    Subtítulo / Linha Fina (Resumo para o Card e Redes Sociais)
                  </label>
                  <input
                    type="text"
                    value={subtitulo}
                    onChange={(e) => setSubtitulo(e.target.value)}
                    placeholder="Síntese objetiva em uma ou duas frases para atrair a leitura no feed..."
                    className="w-full px-4 py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 text-sm"
                  />
                </div>

                {/* Escopo e Seletor Territorial Assistido (Evita erro de código IBGE) */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 p-4 bg-blue-50/50 rounded-xl border border-blue-100">
                  <div>
                    <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1.5">
                      Escopo Federativo
                    </label>
                    <select
                      value={escopo}
                      onChange={(e) =>
                        handleEscopoChange(
                          e.target.value as "MUNICIPAL" | "ESTADUAL" | "NACIONAL"
                        )
                      }
                      className="w-full px-3.5 py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 bg-white text-sm font-medium"
                    >
                      <option value="MUNICIPAL">Municipal (Diretório Local — IBGE 7 dígitos)</option>
                      <option value="ESTADUAL">Estadual (Diretório UF — IBGE 2 dígitos)</option>
                      <option value="NACIONAL">Nacional (Executiva Federal)</option>
                    </select>
                  </div>

                  {escopo === "ESTADUAL" && (
                    <div>
                      <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1.5">
                        Unidade da Federação (UF)
                      </label>
                      <select
                        value={ibgeSelecionado}
                        onChange={(e) => setIbgeSelecionado(e.target.value)}
                        className="w-full px-3.5 py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 bg-white text-sm"
                      >
                        {CATALOGO_ESTADOS_IBGE.map((uf) => (
                          <option key={uf.ibge} value={uf.ibge}>
                            {uf.nome} — IBGE {uf.ibge}
                          </option>
                        ))}
                      </select>
                    </div>
                  )}

                  {escopo === "MUNICIPAL" && (
                    <div className="space-y-2">
                      <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1">
                        Município (Busca Assistida IBGE)
                      </label>
                      <select
                        value={ibgeSelecionado}
                        onChange={(e) => setIbgeSelecionado(e.target.value)}
                        className="w-full px-3.5 py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 bg-white text-sm"
                      >
                        {CATALOGO_MUNICIPIOS_IBGE.map((m) => (
                          <option key={m.ibge} value={m.ibge}>
                            {m.ibge === "OUTRO" ? m.nome : `${m.nome} (${m.ibge})`}
                          </option>
                        ))}
                      </select>

                      {ibgeSelecionado === "OUTRO" && (
                        <input
                          type="text"
                          maxLength={7}
                          value={ibgeCustomizado}
                          onChange={(e) => setIbgeCustomizado(e.target.value.replace(/\D/g, ""))}
                          placeholder="Digite os 7 dígitos do IBGE (Ex: 3548708)"
                          className="w-full px-3 py-1.5 rounded-lg border border-slate-300 text-xs"
                        />
                      )}
                    </div>
                  )}

                  {escopo === "NACIONAL" && (
                    <div className="flex items-center text-xs text-slate-600 bg-white/80 px-3 py-2 rounded-lg border border-slate-200">
                      <MapPin className="w-4 h-4 text-destaque mr-2 flex-shrink-0" />
                      Matéria visível na vitrine nacional para todas as 27 Unidades da Federação.
                    </div>
                  )}
                </div>

                {/* Formato Multimídia + Imagem de Capa */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 p-4 bg-slate-50 rounded-xl border border-slate-200">
                  <div>
                    <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1.5">
                      Formato da Publicação
                    </label>
                    <select
                      value={formato}
                      onChange={(e) =>
                        setFormato(
                          e.target.value as "TEXTO" | "VIDEO" | "AUDIO" | "IMPRESSO_PDF"
                        )
                      }
                      className="w-full px-3.5 py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 bg-white text-sm font-medium"
                    >
                      <option value="TEXTO">Texto (Reportagem / Artigo)</option>
                      <option value="VIDEO">Vídeo (TV Partido / MP4 / HLS)</option>
                      <option value="AUDIO">Áudio (Podcast / Boletim de Rádio)</option>
                      <option value="IMPRESSO_PDF">Paper News (Jornal Impresso em PDF)</option>
                    </select>
                  </div>

                  <div>
                    <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1.5">
                      URL da Imagem de Capa (Opcional)
                    </label>
                    <input
                      type="url"
                      value={imagemDestaqueUrl}
                      onChange={(e) => setImagemDestaqueUrl(e.target.value)}
                      placeholder="https://images.unsplash.com/..."
                      className="w-full px-3.5 py-2 rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 text-sm bg-white"
                    />
                  </div>

                  {formato === "VIDEO" && (
                    <div className="md:col-span-2">
                      <label className="block text-xs font-bold text-rose-700 uppercase tracking-wider mb-1">
                        URL do Vídeo (MP4 / Stream CDN) *
                      </label>
                      <input
                        type="url"
                        required
                        value={videoUrl}
                        onChange={(e) => setVideoUrl(e.target.value)}
                        placeholder="https://cdn.jornalfederado.org.br/videos/pronunciamento.mp4"
                        className="w-full px-3.5 py-2 rounded-lg border border-rose-300 focus:outline-none focus:ring-2 focus:ring-rose-500 text-slate-900 text-sm bg-white"
                      />
                    </div>
                  )}

                  {formato === "AUDIO" && (
                    <div className="md:col-span-2">
                      <label className="block text-xs font-bold text-violet-700 uppercase tracking-wider mb-1">
                        URL do Áudio (MP3 / Podcast) *
                      </label>
                      <input
                        type="url"
                        required
                        value={audioUrl}
                        onChange={(e) => setAudioUrl(e.target.value)}
                        placeholder="https://cdn.jornalfederado.org.br/audios/boletim_semanal.mp3"
                        className="w-full px-3.5 py-2 rounded-lg border border-violet-300 focus:outline-none focus:ring-2 focus:ring-violet-500 text-slate-900 text-sm bg-white"
                      />
                    </div>
                  )}

                  {formato === "IMPRESSO_PDF" && (
                    <div className="md:col-span-2">
                      <label className="block text-xs font-bold text-amber-800 uppercase tracking-wider mb-1">
                        URL da Edição Impressa (Paper News em PDF) *
                      </label>
                      <input
                        type="url"
                        required
                        value={pdfEdicaoUrl}
                        onChange={(e) => setPdfEdicaoUrl(e.target.value)}
                        placeholder="https://cdn.jornalfederado.org.br/jornais/edicao_mensal.pdf"
                        className="w-full px-3.5 py-2 rounded-lg border border-amber-300 focus:outline-none focus:ring-2 focus:ring-amber-500 text-slate-900 text-sm bg-white"
                      />
                    </div>
                  )}
                </div>

                {/* Editor com Barra de Formatação Visual */}
                <div>
                  <div className="flex flex-wrap items-center justify-between gap-2 mb-1.5">
                    <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider">
                      Corpo da Matéria / Transcrição Acessível *
                    </label>

                    <div className="flex items-center gap-1 bg-slate-100 p-1 rounded-lg border border-slate-200">
                      <button
                        type="button"
                        onClick={() => inserirBlocoFormatado("h2")}
                        title="Inserir Subtítulo H2"
                        className="p-1.5 hover:bg-white rounded text-slate-700 transition-colors"
                      >
                        <Heading2 className="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        onClick={() => inserirBlocoFormatado("strong")}
                        title="Negrito"
                        className="p-1.5 hover:bg-white rounded text-slate-700 transition-colors"
                      >
                        <Bold className="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        onClick={() => inserirBlocoFormatado("em")}
                        title="Itálico"
                        className="p-1.5 hover:bg-white rounded text-slate-700 transition-colors"
                      >
                        <Italic className="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        onClick={() => inserirBlocoFormatado("blockquote")}
                        title="Citação Oficial"
                        className="p-1.5 hover:bg-white rounded text-slate-700 transition-colors"
                      >
                        <Quote className="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        onClick={() => inserirBlocoFormatado("ul")}
                        title="Lista de Tópicos"
                        className="p-1.5 hover:bg-white rounded text-slate-700 transition-colors"
                      >
                        <List className="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        onClick={() => inserirBlocoFormatado("a")}
                        title="Inserir Link Seguro"
                        className="p-1.5 hover:bg-white rounded text-slate-700 transition-colors"
                      >
                        <LinkIcon className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>

                  <textarea
                    required
                    rows={10}
                    value={conteudoHtml}
                    onChange={(e) => setConteudoHtml(e.target.value)}
                    placeholder="<p>Redija o conteúdo utilizando os botões de formatação acima...</p>"
                    className="w-full px-4 py-3 rounded-xl border border-slate-300 focus:outline-none focus:ring-2 focus:ring-destaque text-slate-900 font-mono text-xs leading-relaxed"
                  />
                  <p className="text-[11px] text-slate-500 mt-1.5 flex items-center gap-1.5">
                    <ShieldCheck className="w-3.5 h-3.5 text-emerald-600" />
                    Sanitização ativa (Whitelist Ammonia: <code>p, h2, h3, strong, em, ul, ol, li, blockquote, a</code>). Scripts são bloqueados.
                  </p>
                </div>

                {/* Botões de Ação: Rascunho vs. Publicar Agora */}
                <div className="flex flex-wrap items-center justify-between gap-3 pt-4 border-t border-slate-100">
                  <button
                    type="button"
                    disabled={enviando || !titulo.trim()}
                    onClick={() => submeterMateria(false)}
                    className="flex items-center gap-2 bg-slate-100 hover:bg-slate-200 text-slate-800 font-semibold px-4 py-2.5 rounded-xl text-xs transition-colors disabled:opacity-50"
                  >
                    <Save className="w-4 h-4" />
                    Salvar como Rascunho
                  </button>

                  <button
                    type="button"
                    disabled={enviando || !titulo.trim()}
                    onClick={() => submeterMateria(true)}
                    className="flex items-center gap-2 bg-destaque hover:bg-blue-700 text-white font-bold px-6 py-2.5 rounded-xl text-sm shadow-sm transition-colors disabled:opacity-50"
                  >
                    <Send className="w-4 h-4" />
                    {enviando ? "Publicando..." : "Publicar Agora no Feed"}
                  </button>
                </div>
              </div>
            )}

            {/* Coluna de Pré-Visualização ao Vivo (Como o Leitor Verá) */}
            {(modoVisualizacao === "preview" || modoVisualizacao === "split") && (
              <div
                className={`${
                  modoVisualizacao === "split" ? "lg:col-span-5" : "max-w-3xl mx-auto w-full"
                } space-y-5`}
              >
                <div className="bg-slate-900 text-white p-4 rounded-2xl shadow-sm space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] font-bold uppercase tracking-wider text-amber-400 flex items-center gap-1.5">
                      <Sparkles className="w-3.5 h-3.5" />
                      Simulador de Leitura em Tempo Real
                    </span>
                    <span className="text-[11px] text-slate-400">
                      Escopo: {escopo} {codigoIbgeFinal ? `(${codigoIbgeFinal})` : ""}
                    </span>
                  </div>
                  <p className="text-xs text-slate-300">
                    Confira abaixo exatamente como o seu Card aparecerá no Feed e como o Player Multimídia e o texto serão exibidos ao leitor.
                  </p>
                </div>

                {/* Preview 1: Card no Feed */}
                <div className="space-y-2">
                  <span className="text-xs font-bold uppercase tracking-wider text-slate-500">
                    1. Prévia do Card na Vitrine Territorial
                  </span>
                  <CardNoticia
                    noticia={{
                      id: "preview",
                      titulo: titulo || "Título da sua matéria aparecerá aqui em destaque",
                      slug: "preview",
                      subtitulo:
                        subtitulo ||
                        "Subtítulo ou linha fina resumindo o impacto regional para os leitores...",
                      escopo,
                      ibge_codigo: codigoIbgeFinal || undefined,
                      formato,
                      imagem_destaque_url:
                        imagemDestaqueUrl ||
                        "https://images.unsplash.com/photo-1540910419892-4a36d2c3266c?auto=format&fit=crop&w=800&q=80",
                      conteudo_sanitizado: conteudoSanitizadoPreview ?? "",
                      hype_score: 18.5,
                      publicado_em: new Date().toISOString(),
                    }}
                    modoEleitoral={modoEleitoral.ativo}
                  />
                </div>

                {/* Preview 2: Corpo da Matéria + Player Multimídia */}
                <div className="bg-white p-5 rounded-2xl border border-slate-200 shadow-sm space-y-4">
                  <span className="text-xs font-bold uppercase tracking-wider text-slate-500 block border-b border-slate-100 pb-2">
                    2. Prévia da Página Interna & Multimídia
                  </span>

                  <PlayerMultimidia
                    formato={formato}
                    videoUrl={videoUrl || "https://www.w3schools.com/html/mov_bbb.mp4"}
                    audioUrl={audioUrl || "https://www.w3schools.com/html/horse.mp3"}
                    pdfUrl={pdfEdicaoUrl || "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf"}
                    titulo={titulo || "Prévia Multimídia"}
                  />

                  {conteudoSanitizadoPreview !== null ? (
                    <div
                      className="prose prose-sm prose-slate max-w-none prose-headings:font-bold prose-blockquote:border-destaque prose-blockquote:bg-blue-50/50 prose-blockquote:py-1 prose-blockquote:px-3 prose-blockquote:rounded-r-lg"
                      dangerouslySetInnerHTML={{ __html: conteudoSanitizadoPreview }}
                    />
                  ) : (
                    <div className="space-y-2">
                      <p className="text-[11px] text-amber-800 bg-amber-50 border border-amber-200 rounded-md px-2.5 py-1.5">
                        Prévia segura em texto plano (conecte a API Rust/Ammonia para pré-visualização HTML sanitizada).
                      </p>
                      <div className="text-sm text-slate-700 leading-relaxed whitespace-pre-wrap">
                        {conteudoHtml.replace(/<[^>]+>/g, " ").trim() || "Nenhum conteúdo redigido."}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* ABA 2: ACERVO & ESCALONAMENTO FEDERADO (MUNICIPAL -> ESTADUAL -> NACIONAL) */}
      {abaAtiva === "escalonamento" && (
        <div className="space-y-6">
          {/* Fila de Avaliação da Mesa Editora Superior */}
          <div className="bg-white p-6 rounded-2xl border border-slate-200 shadow-sm space-y-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <h2 className="text-lg font-bold text-slate-900">
                  Fila de Avaliação de Escalonamento Federado
                </h2>
                <p className="text-xs text-slate-500">
                  Promoção transacional em degraus estritos: Municipal → Estadual (UF) → Nacional.
                </p>
              </div>
              <button
                type="button"
                onClick={() => void carregarDadosOperacionais()}
                className="inline-flex items-center gap-1.5 text-xs font-bold text-slate-700 bg-slate-100 hover:bg-slate-200 px-3 py-1.5 rounded-lg transition-colors"
              >
                <RefreshCw className="w-3.5 h-3.5" />
                Atualizar Fila
              </button>
            </div>

            {filaEscalonamento.length === 0 ? (
              <p className="text-xs text-slate-500 bg-slate-50 border border-slate-200/80 rounded-xl p-4">
                Nenhuma solicitação de escalonamento registrada para a sua jurisdição no momento.
              </p>
            ) : (
              <div className="divide-y divide-slate-100">
                {filaEscalonamento.map((sol) => (
                  <div key={sol.id} className="py-4 space-y-3">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <span
                          className={`text-[10px] font-bold uppercase px-2 py-0.5 rounded ${
                            sol.status === "PENDENTE"
                              ? "bg-amber-100 text-amber-900"
                              : sol.status === "APROVADO"
                              ? "bg-emerald-100 text-emerald-800"
                              : "bg-rose-100 text-rose-800"
                          }`}
                        >
                          {sol.status}
                        </span>
                        <span className="text-xs font-bold text-indigo-800 bg-indigo-50 px-2.5 py-0.5 rounded">
                          {sol.escopo_origem} → {sol.escopo_destino}
                        </span>
                        <span className="text-xs text-slate-500">
                          Solicitante: <strong>{sol.solicitante_nome}</strong>
                        </span>
                      </div>
                      <span className="text-[11px] text-slate-400">
                        {new Date(sol.criado_em).toLocaleString("pt-BR")}
                      </span>
                    </div>

                    <div>
                      <h3 className="text-sm font-bold text-slate-900">{sol.noticia_titulo}</h3>
                      <p className="text-xs text-slate-600 mt-0.5">
                        Justificativa: {sol.justificativa}
                      </p>
                      {sol.parecer_revisao && (
                        <p className="text-xs text-slate-500 mt-1 italic">
                          Parecer ({sol.avaliado_por_nome || "Mesa Editora"}): {sol.parecer_revisao}
                        </p>
                      )}
                    </div>

                    {sol.status === "PENDENTE" &&
                      sessaoAtual?.permissoes.pode_aprovar_escalonamento && (
                        <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-2 pt-1">
                          <input
                            type="text"
                            value={parecerPorSolicitacao[sol.id] || ""}
                            onChange={(e) =>
                              setParecerPorSolicitacao((prev) => ({
                                ...prev,
                                [sol.id]: e.target.value,
                              }))
                            }
                            placeholder="Parecer editorial opcional para registro em auditoria..."
                            className="flex-1 px-3 py-1.5 rounded-lg border border-slate-300 text-xs text-slate-900"
                          />
                          <button
                            type="button"
                            disabled={avaliandoId === sol.id}
                            onClick={() => void handleAvaliarEscalonamento(sol.id, true)}
                            className="inline-flex items-center justify-center gap-1.5 text-xs font-bold bg-emerald-600 hover:bg-emerald-700 text-white px-3.5 py-1.5 rounded-lg transition-colors disabled:opacity-50"
                          >
                            <CheckCircle2 className="w-3.5 h-3.5" />
                            Aprovar Promoção
                          </button>
                          <button
                            type="button"
                            disabled={avaliandoId === sol.id}
                            onClick={() => void handleAvaliarEscalonamento(sol.id, false)}
                            className="inline-flex items-center justify-center gap-1.5 text-xs font-bold bg-rose-50 hover:bg-rose-100 text-rose-800 border border-rose-200 px-3.5 py-1.5 rounded-lg transition-colors disabled:opacity-50"
                          >
                            <XCircle className="w-3.5 h-3.5" />
                            Rejeitar
                          </button>
                        </div>
                      )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Acervo Real da Jurisdição */}
          <div className="bg-white p-6 rounded-2xl border border-slate-200 shadow-sm space-y-5">
            <div>
              <h2 className="text-lg font-bold text-slate-900">
                Acervo do Diretório & Indicação de Escalonamento
              </h2>
              <p className="text-xs text-slate-500">
                Matérias locais podem ser escalonadas ao Diretório Estadual, e matérias estaduais podem ser escalonadas à Executiva Nacional.
              </p>
            </div>

            <div className="divide-y divide-slate-100">
              {acervo.map((item) => (
                <div
                  key={item.id}
                  className="py-4 flex flex-col md:flex-row md:items-center justify-between gap-4"
                >
                  <div className="space-y-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span
                        className={`text-[10px] font-bold uppercase px-2 py-0.5 rounded ${
                          item.status === "PUBLICADO"
                            ? "bg-emerald-100 text-emerald-800"
                            : "bg-slate-100 text-slate-600"
                        }`}
                      >
                        {item.status}
                      </span>
                      <span className="text-[11px] font-bold text-destaque bg-blue-50 px-2 py-0.5 rounded">
                        {item.escopo} {item.ibge_codigo ? `• IBGE ${item.ibge_codigo}` : ""}
                      </span>
                      <span className="text-[11px] font-semibold text-amber-700 bg-amber-50 px-2 py-0.5 rounded flex items-center gap-1">
                        <Flame className="w-3 h-3 fill-amber-500 text-amber-500" />
                        {item.hype_score.toFixed(1)} Hype
                      </span>
                    </div>

                    <h3 className="text-base font-bold text-slate-900">{item.titulo}</h3>
                    <p className="text-xs text-slate-400">Slug: /noticia/{item.slug}</p>
                  </div>

                  <div className="flex items-center gap-2 self-start md:self-center">
                    {item.escalonamento_solicitado ? (
                      <span className="text-xs font-semibold text-emerald-700 bg-emerald-50 border border-emerald-200 px-3 py-1.5 rounded-lg flex items-center gap-1.5">
                        <CheckCircle2 className="w-4 h-4" />
                        Pendente p/ Vitrine {item.escalonamento_solicitado}
                      </span>
                    ) : (
                      <>
                        {item.escopo === "MUNICIPAL" && (
                          <button
                            type="button"
                            onClick={() => void handleSolicitarEscalonamento(item.id, "ESTADUAL")}
                            className="flex items-center gap-1.5 text-xs font-bold bg-indigo-50 hover:bg-indigo-100 text-indigo-800 border border-indigo-200 px-3 py-2 rounded-lg transition-colors"
                          >
                            <ArrowUpRight className="w-4 h-4" />
                            Escalonar p/ Estadual
                          </button>
                        )}
                        {item.escopo === "ESTADUAL" && (
                          <button
                            type="button"
                            onClick={() => void handleSolicitarEscalonamento(item.id, "NACIONAL")}
                            className="flex items-center gap-1.5 text-xs font-bold bg-amber-50 hover:bg-amber-100 text-amber-900 border border-amber-200 px-3 py-2 rounded-lg transition-colors"
                          >
                            <ArrowUpRight className="w-4 h-4" />
                            Escalonar p/ Nacional
                          </button>
                        )}
                      </>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* ABA 3: GOVERNANÇA DE FILIADOS & KILL-SWITCH DE DISSIDÊNCIA (ADR D2) */}
      {abaAtiva === "governanca" && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Card 1: Habilitação de Filiados Votantes com Cota Semanal */}
          <div className="bg-white p-6 rounded-2xl border border-slate-200 shadow-sm space-y-5">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-lg font-bold text-slate-900 flex items-center gap-2">
                  <Users className="w-5 h-5 text-destaque" />
                  Habilitar Filiados para Votação (ADR D2)
                </h2>
                <p className="text-xs text-slate-500">
                  Apenas filiados validados pela liderança podem votar no ranking Hype.
                </p>
              </div>
              <span className="text-xs font-black bg-blue-50 text-destaque px-3 py-1.5 rounded-full border border-blue-200">
                {cotaAtivacoesUsada} / 50 na semana
              </span>
            </div>

            {/* Barra de Progresso da Cota Antifraude */}
            <div className="space-y-1.5">
              <div className="w-full h-2.5 bg-slate-100 rounded-full overflow-hidden">
                <div
                  className="h-full bg-destaque transition-all duration-300"
                  style={{ width: `${Math.min(100, (cotaAtivacoesUsada / 50) * 100)}%` }}
                />
              </div>
              <p className="text-[11px] text-slate-400">
                Trava antifraude: cada liderança possui limite de 50 ativações semanais auditadas.
              </p>
            </div>

            <form onSubmit={handleAtivarFiliado} className="space-y-4 pt-2">
              <div>
                <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1">
                  E-mail Institucional ou Pessoal do Filiado *
                </label>
                <input
                  type="email"
                  required
                  value={emailNovoFiliado}
                  onChange={(e) => setEmailNovoFiliado(e.target.value)}
                  placeholder="filiado.verificado@agremiacao.org.br"
                  className="w-full px-3.5 py-2 rounded-lg border border-slate-300 text-sm"
                />
              </div>

              <div>
                <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1">
                  Inscrição Partidária / CPF (Hash SHA-256 no Backend) *
                </label>
                <input
                  type="text"
                  required
                  value={cpfHashNovoFiliado}
                  onChange={(e) => setCpfHashNovoFiliado(e.target.value)}
                  placeholder="000.000.000-00"
                  className="w-full px-3.5 py-2 rounded-lg border border-slate-300 text-sm"
                />
              </div>

              <button
                type="submit"
                disabled={cotaAtivacoesUsada >= 50}
                className="w-full bg-destaque hover:bg-blue-700 text-white font-bold py-2.5 rounded-xl text-xs transition-colors disabled:opacity-50"
              >
                Habilitar Direito de Voto Hype (1 Pessoa = 1 Voto)
              </button>
            </form>
          </div>

          {/* Card 2: Kill-Switch de Dissidência Partidária */}
          <div className="bg-rose-50/50 p-6 rounded-2xl border border-rose-200 shadow-sm space-y-5">
            <div>
              <h2 className="text-lg font-bold text-rose-950 flex items-center gap-2">
                <AlertOctagon className="w-5 h-5 text-rose-600" />
                Kill-Switch de Dissidência Partidária
              </h2>
              <p className="text-xs text-rose-800 mt-1">
                Revogação atômica imediata de todas as contas e sessões ativas subordinadas a uma liderança ou diretório que tenha deixado a sigla.
              </p>
            </div>

            <form onSubmit={handleKillSwitchRevogacao} className="space-y-4">
              <div>
                <label className="block text-xs font-bold text-rose-900 uppercase tracking-wider mb-1">
                  UUID ou E-mail da Liderança / Diretório em Dissidência *
                </label>
                <input
                  type="text"
                  required
                  value={liderDissidenteAlvo}
                  onChange={(e) => setLiderDissidenteAlvo(e.target.value)}
                  placeholder="ex.: lider.dissidente@partido.org.br ou UUID"
                  className="w-full px-3.5 py-2 rounded-lg border border-rose-300 bg-white text-sm text-slate-900"
                />
              </div>

              <div>
                <label className="block text-xs font-bold text-rose-900 uppercase tracking-wider mb-1">
                  Justificativa Formal para o Log Imutável de Auditoria *
                </label>
                <textarea
                  rows={4}
                  required
                  value={motivoKillSwitch}
                  onChange={(e) => setMotivoKillSwitch(e.target.value)}
                  placeholder="Ex: Desfiliação comunicada em ata oficial do Diretório Estadual..."
                  className="w-full px-3.5 py-2 rounded-lg border border-rose-300 bg-white text-sm text-slate-900"
                />
              </div>

              <button
                type="submit"
                className="w-full bg-rose-600 hover:bg-rose-700 text-white font-bold py-2.5 rounded-xl text-xs shadow-sm transition-colors"
              >
                Executar Revogação em Cascata & Expurgar Sessões Ativas
              </button>
            </form>
          </div>
        </div>
      )}

      {/* ABA 4: SEGURANÇA 2FA, INTERRUPTOR MODO ELEITORAL & AUDITORIA */}
      {abaAtiva === "seguranca" && (
        <div className="space-y-6">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Card 1: Interruptor Visual de Modo Eleitoral (Lei nº 9.504/97 — ADR D6) */}
            <div className="bg-white p-6 rounded-2xl border border-slate-200 shadow-sm space-y-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h2 className="text-lg font-bold text-slate-900 flex items-center gap-2">
                    <Scale className="w-5 h-5 text-amber-600" />
                    Modo Eleitoral (Lei nº 9.504/97 — ADR D6)
                  </h2>
                  <p className="text-xs text-slate-500 mt-1">
                    Congela a ordenação algorítmica Hype no feed público, oculta placares e fixa o banner jurídico obrigatório.
                  </p>
                </div>
                <span
                  className={`text-[11px] font-black uppercase px-3 py-1 rounded-full border ${
                    modoEleitoral.ativo
                      ? "bg-amber-100 text-amber-900 border-amber-300"
                      : "bg-emerald-50 text-emerald-800 border-emerald-200"
                  }`}
                >
                  {modoEleitoral.ativo ? "ATIVO (FREEZE)" : "ORDINÁRIO"}
                </span>
              </div>

              <div className="p-3.5 rounded-xl bg-slate-50 border border-slate-200 text-xs space-y-1">
                <p className="font-semibold text-slate-700">
                  Último motivo registrado: <span className="font-normal">{modoEleitoral.motivo}</span>
                </p>
                <p className="text-slate-500">Banner: {modoEleitoral.banner_texto}</p>
              </div>

              <div className="space-y-2">
                <label className="block text-xs font-bold text-slate-700 uppercase tracking-wider">
                  Justificativa Jurídica / Operacional para Auditoria *
                </label>
                <input
                  type="text"
                  value={motivoModoEleitoral}
                  onChange={(e) => setMotivoModoEleitoral(e.target.value)}
                  placeholder="Ex: Início do período defeso eleitoral conforme resolução TSE..."
                  className="w-full px-3.5 py-2 rounded-lg border border-slate-300 text-sm text-slate-900"
                />
              </div>

              <div className="flex flex-wrap items-center gap-3 pt-1">
                <button
                  type="button"
                  disabled={
                    salvandoModoEleitoral ||
                    !(
                      sessaoAtual?.permissoes.pode_gerenciar_infra ||
                      (sessaoAtual?.permissoes.pode_alterar_cargos &&
                        sessaoAtual?.escopo === "NACIONAL")
                    )
                  }
                  onClick={() => void handleAlternarModoEleitoral(!modoEleitoral.ativo)}
                  className={`flex-1 font-bold py-2.5 px-4 rounded-xl text-xs transition-colors disabled:opacity-50 ${
                    modoEleitoral.ativo
                      ? "bg-emerald-600 hover:bg-emerald-700 text-white"
                      : "bg-amber-600 hover:bg-amber-700 text-white"
                  }`}
                >
                  {salvandoModoEleitoral
                    ? "Gravando em configuracoes..."
                    : modoEleitoral.ativo
                    ? "Restaurar Operação Ordinária (Reativar Hype)"
                    : "Ativar Modo Eleitoral (Congelar Hype Agora)"}
                </button>
              </div>
              <p className="text-[11px] text-slate-400">
                Restrito à Custódia Nacional de Infraestrutura (SuperAdmin) ou Presidência Executiva Nacional.
              </p>
            </div>

            {/* Card 2: Setup Automático de 2FA TOTP SHA-256 */}
            <div className="bg-white p-6 rounded-2xl border border-slate-200 shadow-sm space-y-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h2 className="text-lg font-bold text-slate-900 flex items-center gap-2">
                    <KeyRound className="w-5 h-5 text-destaque" />
                    Segundo Fator 2FA TOTP (HMAC-SHA256)
                  </h2>
                  <p className="text-xs text-slate-500 mt-1">
                    Provisionamento em duas etapas com envelope cifrado AES-256-GCM e 8 códigos de recuperação de uso único.
                  </p>
                </div>
                <span
                  className={`text-[11px] font-black uppercase px-3 py-1 rounded-full border ${
                    sessaoAtual?.totp_habilitado
                      ? "bg-emerald-50 text-emerald-800 border-emerald-200"
                      : "bg-slate-100 text-slate-700 border-slate-300"
                  }`}
                >
                  {sessaoAtual?.totp_habilitado ? "2FA ATIVO" : "2FA INATIVO"}
                </span>
              </div>

              {codigosRecuperacaoGerados && codigosRecuperacaoGerados.length > 0 && (
                <div className="p-4 rounded-xl bg-amber-50 border border-amber-300 space-y-2">
                  <p className="text-xs font-bold text-amber-950">
                    Guarde seus 8 códigos de recuperação (exibidos apenas uma vez):
                  </p>
                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 font-mono text-xs font-bold text-slate-900">
                    {codigosRecuperacaoGerados.map((cod) => (
                      <span
                        key={cod}
                        className="bg-white border border-amber-200 rounded px-2 py-1 text-center"
                      >
                        {cod}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {!sessaoAtual?.totp_habilitado && !setup2faDados && (
                <button
                  type="button"
                  disabled={!sessaoAtual || processando2fa}
                  onClick={() => void handleIniciarSetup2fa()}
                  className="w-full bg-destaque hover:bg-blue-700 text-white font-bold py-2.5 rounded-xl text-xs transition-colors disabled:opacity-50"
                >
                  {processando2fa
                    ? "Gerando envelope AES-256-GCM..."
                    : "Iniciar Provisionamento 2FA TOTP"}
                </button>
              )}

              {setup2faDados && (
                <form onSubmit={handleConfirmarSetup2fa} className="space-y-3">
                  <div className="p-3 rounded-xl bg-slate-900 text-slate-100 text-xs space-y-1.5 font-mono break-all">
                    <div>
                      <span className="text-slate-400">Segredo Base32: </span>
                      <span className="text-amber-300 font-bold">{setup2faDados.secret_base32}</span>
                    </div>
                    <div className="text-[11px] text-slate-300">{setup2faDados.otpauth_url}</div>
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="text"
                      inputMode="numeric"
                      maxLength={6}
                      required
                      value={codigoConfirmacao2fa}
                      onChange={(e) => setCodigoConfirmacao2fa(e.target.value.replace(/\D/g, ""))}
                      placeholder="Código de 6 dígitos"
                      className="flex-1 px-3.5 py-2 rounded-lg border border-slate-300 text-sm font-mono text-slate-900"
                    />
                    <button
                      type="submit"
                      disabled={processando2fa || codigoConfirmacao2fa.length !== 6}
                      className="bg-emerald-600 hover:bg-emerald-700 text-white font-bold px-4 py-2 rounded-lg text-xs transition-colors disabled:opacity-50"
                    >
                      Confirmar & Ativar
                    </button>
                  </div>
                </form>
              )}

              {sessaoAtual?.totp_habilitado && !codigosRecuperacaoGerados && (
                <p className="text-xs text-emerald-800 bg-emerald-50 border border-emerald-200 rounded-xl p-3.5">
                  Esta conta já exige segundo fator TOTP SHA-256 em todos os logins administrativos.
                </p>
              )}
            </div>
          </div>

          {/* Card 3: Sessões HTTP-Only Ativas & Trilha de Auditoria */}
          <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
            <div className="lg:col-span-5 bg-white p-6 rounded-2xl border border-slate-200 shadow-sm space-y-4">
              <div className="flex items-center justify-between gap-2">
                <h3 className="text-base font-bold text-slate-900 flex items-center gap-2">
                  <Laptop className="w-4 h-4 text-destaque" />
                  Sessões HTTP-Only Ativas ({sessoesAtivas.length})
                </h3>
                {sessoesAtivas.length > 1 && (
                  <button
                    type="button"
                    onClick={() => void handleRevogarOutrasSessoes()}
                    className="text-xs font-bold text-rose-700 hover:underline"
                  >
                    Encerrar outras sessões
                  </button>
                )}
              </div>

              {sessoesAtivas.length === 0 ? (
                <p className="text-xs text-slate-500">
                  Faça login editorial para inspecionar suas sessões ativas.
                </p>
              ) : (
                <div className="divide-y divide-slate-100">
                  {sessoesAtivas.map((s, idx) => (
                    <div key={`${s.criado_em}-${idx}`} className="py-2.5 text-xs space-y-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-mono font-semibold text-slate-800">{s.ip_origem}</span>
                        {s.sessao_atual && (
                          <span className="text-[10px] font-bold bg-emerald-100 text-emerald-800 px-2 py-0.5 rounded">
                            Sessão Atual
                          </span>
                        )}
                      </div>
                      <p className="text-slate-500 truncate">{s.user_agent}</p>
                      <p className="text-[11px] text-slate-400">
                        Último acesso: {new Date(s.ultimo_acesso).toLocaleString("pt-BR")}
                      </p>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="lg:col-span-7 bg-white p-6 rounded-2xl border border-slate-200 shadow-sm space-y-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <h3 className="text-base font-bold text-slate-900 flex items-center gap-2">
                    <History className="w-4 h-4 text-destaque" />
                    Trilha Imutável de Auditoria ({logsAuditoria.length})
                  </h3>
                  <p className="text-[11px] text-slate-500">
                    {visaoGlobalAuditoria
                      ? "Visão global de custódia nacional (todos os diretórios)."
                      : "Exibindo eventos autenticados da sua conta."}
                  </p>
                </div>
                <button
                  type="button"
                  disabled={carregandoPainelSeguranca}
                  onClick={() => void carregarDadosOperacionais()}
                  className="inline-flex items-center gap-1.5 text-xs font-bold text-slate-700 bg-slate-100 hover:bg-slate-200 px-3 py-1.5 rounded-lg transition-colors"
                >
                  <RefreshCw className="w-3.5 h-3.5" />
                  Recarregar
                </button>
              </div>

              {logsAuditoria.length === 0 ? (
                <p className="text-xs text-slate-500">
                  Nenhum registro de auditoria carregado nesta sessão.
                </p>
              ) : (
                <div className="max-h-80 overflow-y-auto divide-y divide-slate-100 pr-1">
                  {logsAuditoria.map((log) => (
                    <div key={log.id} className="py-2.5 text-xs flex items-start justify-between gap-3">
                      <div className="space-y-0.5">
                        <div className="flex flex-wrap items-center gap-1.5">
                          <span className="font-mono font-bold text-slate-900 bg-slate-100 px-1.5 py-0.5 rounded">
                            {log.acao}
                          </span>
                          <span className="text-slate-500">em {log.entidade}</span>
                        </div>
                        <p className="text-[11px] text-slate-500">
                          Ator: {log.ator_nome || "Sistema / Pré-login"} • IP {log.ip_origem}
                        </p>
                      </div>
                      <span className="text-[11px] text-slate-400 whitespace-nowrap">
                        {new Date(log.criado_em).toLocaleString("pt-BR")}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
