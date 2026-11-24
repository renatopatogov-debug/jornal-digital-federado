export interface PermissoesSessao {
  pode_gerenciar_infra: boolean;
  pode_alterar_cargos: boolean;
  pode_ativar_filiados: boolean;
  pode_revogar_lote: boolean;
  pode_publicar_nacional: boolean;
  pode_publicar_estadual: boolean;
  pode_publicar_municipal: boolean;
  pode_solicitar_escalonamento: boolean;
  pode_aprovar_escalonamento: boolean;
  pode_votar_hype: boolean;
  limite_ativacao_semanal: number | null;
}

export interface SessaoUsuario {
  autenticado: boolean;
  usuario_id: string;
  nome: string;
  email: string;
  escopo: "NACIONAL" | "ESTADUAL" | "MUNICIPAL";
  ibge_codigo?: string | null;
  filiado_verificado: boolean;
  totp_habilitado?: boolean;
  status: "ATIVO" | "SUSPENSO" | "PENDENTE";
  cargo_slug: string;
  cargo_nome: string;
  nivel_hierarquico: number;
  permissoes: PermissoesSessao;
}

export function obterApiUrl(): string {
  return process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
}

export async function buscarSessaoAtual(signal?: AbortSignal): Promise<SessaoUsuario | null> {
  try {
    const res = await fetch(`${obterApiUrl()}/api/v1/auth/me`, {
      method: "GET",
      credentials: "include",
      cache: "no-store",
      signal,
    });
    if (!res.ok) return null;
    const data = (await res.json().catch(() => null)) as SessaoUsuario | null;
    return data?.autenticado ? data : null;
  } catch {
    return null;
  }
}

export async function encerrarSessao(): Promise<void> {
  try {
    await fetch(`${obterApiUrl()}/api/v1/auth/logout`, {
      method: "POST",
      credentials: "include",
    });
  } catch {
    // Segue limpeza local mesmo se a API estiver momentaneamente offline
  }
}
