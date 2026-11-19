// =============================================================================
// SERVICE WORKER — JORNAL DIGITAL FEDERADO (PWA OFFLINE CACHE SEGURO)
// =============================================================================

const CACHE_NAME = "jornal-federado-v2";
const OFFLINE_URLS = [
  "/",
  "/manifest.webmanifest",
  "/geo/brasil_ufs.json",
  "/icon-192x192.png",
  "/icon-512x512.png"
];

// Instalação do Service Worker e pré-cache estático
self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(OFFLINE_URLS))
  );
  self.skipWaiting();
});

// Ativação e purga de caches antigos (remove caches v1 potencialmente com /api/)
self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keyList) =>
      Promise.all(
        keyList.map((key) => {
          if (key !== CACHE_NAME) {
            return caches.delete(key);
          }
          return undefined;
        })
      )
    )
  );
  self.clients.claim();
});

// Network-First seguro: NUNCA cacheia /api/, /auth/, /cms ou origens externas
self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") return;

  const url = new URL(event.request.url);
  if (url.protocol !== "http:" && url.protocol !== "https:") return;
  if (url.origin !== self.location.origin) return;

  if (
    url.pathname.startsWith("/api/") ||
    url.pathname.startsWith("/auth/") ||
    url.pathname.startsWith("/cms")
  ) {
    return;
  }

  event.respondWith(
    fetch(event.request)
      .then((networkResponse) => {
        const cacheControl = networkResponse.headers.get("cache-control") || "";
        const podeCachear =
          networkResponse.ok &&
          networkResponse.type === "basic" &&
          !cacheControl.toLowerCase().includes("no-store");

        if (podeCachear) {
          const responseToCache = networkResponse.clone();
          caches.open(CACHE_NAME).then((cache) => {
            cache.put(event.request, responseToCache);
          });
        }
        return networkResponse;
      })
      .catch(async () => {
        const cached = await caches.match(event.request);
        if (cached) return cached;
        if (event.request.mode === "navigate") {
          const home = await caches.match("/");
          if (home) return home;
        }
        return Response.error();
      })
  );
});
