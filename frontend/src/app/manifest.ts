import { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Jornal Digital Federado",
    short_name: "Jornal Federado",
    description: "Plataforma editorial federada aberta com ranking auditável e notícias hiperlocais de todo o Brasil.",
    start_url: "/",
    display: "standalone",
    background_color: "#f7f2e4",
    theme_color: "#8b261d",
    icons: [
      {
        src: "/icon-192x192.png",
        sizes: "192x192",
        type: "image/png",
        purpose: "any",
      },
      {
        src: "/icon-512x512.png",
        sizes: "512x512",
        type: "image/png",
        purpose: "maskable",
      },
    ],
  };
}
