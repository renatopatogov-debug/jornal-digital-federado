import type { Metadata } from "next";
import "./globals.css";
import { BannerEleitoral } from "@/components/BannerEleitoral";

export const metadata: Metadata = {
  title: "Jornal Digital Federado — Notícias e Participação",
  description: "Plataforma editorial federada aberta com ranking auditável e neutralidade eleitoral estrita (Lei nº 9.504/97).",
  manifest: "/manifest.webmanifest",
  themeColor: "#2563eb",
  appleWebApp: {
    capable: true,
    statusBarStyle: "default",
    title: "Jornal Federado",
  },
};

async function getModoEleitoralStatus() {
  const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
  try {
    const res = await fetch(`${apiUrl}/api/v1/feed?limit=1`, {
      next: { revalidate: 30 },
    });
    if (res.ok) {
      const data = await res.json();
      return {
        ativo: data.modo_eleitoral_ativo,
        bannerTexto: data.banner_legal,
      };
    }
  } catch (err) {
    // Modo resiliente offline
  }
  return { ativo: false, bannerTexto: undefined };
}

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { ativo, bannerTexto } = await getModoEleitoralStatus();

  return (
    <html lang="pt-BR">
      <body className="min-h-screen flex flex-col bg-slate-50 text-slate-800 antialiased">
        {/* Banner Institucional Obrigatório do TSE */}
        <BannerEleitoral ativo={ativo} mensagem={bannerTexto} />

        {/* Topbar de Navegação Federada */}
        <header className="bg-slate-900 text-white border-b border-slate-800 sticky top-0 z-40">
          <div className="max-w-7xl mx-auto px-4 h-16 flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-lg bg-destaque flex items-center justify-center font-bold text-white shadow-sm">
                JF
              </div>
              <a href="/" className="font-extrabold text-lg tracking-tight hover:opacity-90">
                Jornal Digital <span className="text-blue-400">Federado</span>
              </a>
            </div>

            <nav className="flex items-center gap-6 text-sm font-medium">
              <a href="/" className="text-slate-300 hover:text-white transition-colors">
                Feeds
              </a>
              <a href="/territorio" className="text-slate-300 hover:text-white transition-colors">
                Mapa Brasil
              </a>
              <a href="/cms" className="text-blue-400 hover:text-blue-300 transition-colors">
                Área de Redação (CMS)
              </a>
              <a
                href="/auth/login"
                className="bg-destaque hover:bg-blue-700 text-white px-3.5 py-1.5 rounded-lg text-xs font-semibold shadow-sm transition-colors"
              >
                Entrar
              </a>
            </nav>
          </div>
        </header>

        {/* Conteúdo Principal */}
        <main className="flex-1">{children}</main>

        {/* Rodapé Institucional */}
        <footer className="bg-white border-t border-slate-200 py-8 mt-12 text-center text-xs text-slate-500">
          <div className="max-w-7xl mx-auto px-4 space-y-2">
            <p className="font-semibold text-slate-700">
              Jornal Digital Federado — Infraestrutura Democrática e Descentralizada
            </p>
            <p>
              Em estrita conformidade com a Lei Geral de Proteção de Dados (LGPD) e Legislação Eleitoral (Lei nº 9.504/97).
            </p>
          </div>
        </footer>

        {/* Registro do Service Worker (PWA Offline) */}
        <script
          dangerouslySetInnerHTML={{
            __html: `
              if ('serviceWorker' in navigator) {
                window.addEventListener('load', function() {
                  navigator.serviceWorker.register('/sw.js').then(
                    function(registration) {
                      console.log('ServiceWorker PWA registrado com escopo: ', registration.scope);
                    },
                    function(err) {
                      console.log('Falha ao registrar ServiceWorker: ', err);
                    }
                  );
                });
              }
            `,
          }}
        />
      </body>
    </html>
  );
}
