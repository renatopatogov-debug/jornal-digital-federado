import http from 'k6/http';
import { check, sleep } from 'k6';

// =============================================================================
// TESTE DE CARGA K6 — 50.000 REQUISIÇÕES POR MINUTO (~833 req/s)
// Plano Diretor v2.1 — Fase 4: Engine Hype, Worker e Carga
//
// Variáveis de ambiente úteis em ambiente de staging/benchmark:
//   API_URL          URL base da API (padrão: http://localhost:8080)
//   NOTICIA_ID       UUID da pauta publicada alvo (autodescoberta via /feed se omitida)
//   SESSION_COOKIE   Cookie HTTP-Only já autenticado (ex: jornal_session=...)
//   FILIADO_EMAIL    Alternativa: autentica em setup() via POST /api/v1/auth/login
//   FILIADO_PASSWORD Senha correspondente para login no setup()
//   DISTRIBUTED_IPS  "true" para simular IPs por VU via X-Forwarded-For (requer TRUST_PROXY=true)
// =============================================================================

export const options = {
  scenarios: {
    // Rampa de subida até atingir 833 req/s contínuas (50k req/min)
    carga_sustentada: {
      executor: 'ramping-arrival-rate',
      startRate: 100,
      timeUnit: '1s',
      preAllocatedVUs: 200,
      maxVUs: 1000,
      stages: [
        { duration: '30s', target: 400 },  // Aquecimento
        { duration: '1m', target: 834 },   // Atinge 50.000 req/min
        { duration: '3m', target: 834 },   // Mantém 50.000 req/min contínuas
        { duration: '30s', target: 0 },    // Resfriamento
      ],
    },
  },
  thresholds: {
    // 95% das requisições devem responder abaixo de 50ms (Cache / SSR)
    http_req_duration: ['p(95)<50', 'p(99)<150'],
    // Taxa de falhas inesperadas inferior a 0.01%
    http_req_failed: ['rate<0.0001'],
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';
const DISTRIBUTED_IPS = __ENV.DISTRIBUTED_IPS === 'true';

const VOTE_CALLBACK = http.expectedStatuses(200, 400, 429);
const UNAUTH_VOTE_CALLBACK = http.expectedStatuses(401);

export function setup() {
  let noticiaId = __ENV.NOTICIA_ID || '';
  let sessionCookie = __ENV.SESSION_COOKIE || '';

  if (!noticiaId) {
    const feedRes = http.get(`${BASE_URL}/api/v1/feed?limit=1`);
    if (feedRes.status === 200) {
      const body = feedRes.json();
      if (body && Array.isArray(body.noticias) && body.noticias.length > 0) {
        noticiaId = body.noticias[0].id;
      }
    }
  }

  if (!sessionCookie && __ENV.FILIADO_EMAIL && __ENV.FILIADO_PASSWORD) {
    const loginRes = http.post(
      `${BASE_URL}/api/v1/auth/login`,
      JSON.stringify({
        email: __ENV.FILIADO_EMAIL,
        senha: __ENV.FILIADO_PASSWORD,
      }),
      { headers: { 'Content-Type': 'application/json' } }
    );
    const setCookieHeader = loginRes.headers['Set-Cookie'] || loginRes.headers['set-cookie'];
    if (loginRes.status === 200 && setCookieHeader) {
      const firstCookie = Array.isArray(setCookieHeader) ? setCookieHeader[0] : setCookieHeader;
      sessionCookie = String(firstCookie).split(';')[0].trim();
    }
  }

  return {
    noticiaId: noticiaId || '00000000-0000-0000-0000-000000000001',
    sessionCookie,
  };
}

function buildHeaders(extraHeaders) {
  const headers = Object.assign({}, extraHeaders || {});
  if (DISTRIBUTED_IPS) {
    const octet3 = Math.floor((__VU - 1) / 250) % 255;
    const octet4 = ((__VU - 1) % 250) + 1;
    headers['X-Forwarded-For'] = `10.0.${octet3}.${octet4}`;
  }
  return headers;
}

export default function (data) {
  const roll = Math.random();

  // Cenário 1 (80%): Leitura Pública de Feeds
  if (roll < 0.80) {
    const res = http.get(`${BASE_URL}/api/v1/feed?limit=20`, {
      headers: buildHeaders(),
    });
    check(res, {
      'Feed status 200': (r) => r.status === 200,
      'Feed response time < 50ms': (r) => r.timings.duration < 50,
    });
  }
  // Cenário 2 (15%): Votação no Ledger (autenticada via cookie HTTP-Only, ou verifica bloqueio 401)
  else if (roll < 0.95) {
    const payload = JSON.stringify({ tipo_voto: 1 });

    if (data.sessionCookie) {
      const res = http.post(
        `${BASE_URL}/api/v1/noticias/${data.noticiaId}/voto`,
        payload,
        {
          headers: buildHeaders({
            'Content-Type': 'application/json',
            Cookie: data.sessionCookie,
          }),
          responseCallback: VOTE_CALLBACK,
        }
      );
      check(res, {
        'Voto autenticado tratado (200/400/429)': (r) =>
          r.status === 200 || r.status === 400 || r.status === 429,
        'Voto response time < 150ms': (r) => r.timings.duration < 150,
      });
    } else {
      const res = http.post(
        `${BASE_URL}/api/v1/noticias/${data.noticiaId}/voto`,
        payload,
        {
          headers: buildHeaders({ 'Content-Type': 'application/json' }),
          responseCallback: UNAUTH_VOTE_CALLBACK,
        }
      );
      check(res, {
        'Voto sem sessao bloqueado com 401': (r) => r.status === 401,
      });
    }
  }
  // Cenário 3 (5%): Consulta de Integridade / Saúde
  else {
    const res = http.get(`${BASE_URL}/health`, {
      headers: buildHeaders(),
    });
    check(res, {
      'Health check status 200': (r) => r.status === 200,
    });
  }

  sleep(0.1);
}
