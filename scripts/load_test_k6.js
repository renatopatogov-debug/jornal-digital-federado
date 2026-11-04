import http from 'k6/http';
import { check, sleep } from 'k6';

// =============================================================================
// TESTE DE CARGA K6 — 50.000 REQUISIÇÕES POR MINUTO (~833 req/s)
// Plano Diretor v2.1 — Fase 4: Engine Hype, Worker e Carga
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
    // 95% das requisições devem responder abaixo de 50ms (Cache Redis / SSR)
    http_req_duration: ['p(95)<50', 'p(99)<150'],
    // Taxa de falhas deve ser inferior a 0.01%
    http_req_failed: ['rate<0.0001'],
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function () {
  const roll = Math.random();

  // Cenário 1 (80%): Leitura Pública de Feeds (Cache em Memória)
  if (roll < 0.80) {
    const res = http.get(`${BASE_URL}/api/v1/feed?limit=20`);
    check(res, {
      'Feed status 200': (r) => r.status === 200,
      'Feed response time < 50ms': (r) => r.timings.duration < 50,
    });
  }
  // Cenário 2 (15%): Votação Concorrente no Ledger de Votos
  else if (roll < 0.95) {
    const noticiaId = '00000000-0000-0000-0000-000000000001';
    const payload = JSON.stringify({
      usuario_id: '00000000-0000-0000-0000-000000000002',
      tipo_voto: 1,
    });

    const params = {
      headers: { 'Content-Type': 'application/json' },
    };

    const res = http.post(`${BASE_URL}/api/v1/noticias/${noticiaId}/voto`, payload, params);
    // Aceita 200 (sucesso) ou 400 (voto já computado pela restrição de unicidade)
    check(res, {
      'Voto tratado corretamente (200 ou 400)': (r) => r.status === 200 || r.status === 400,
      'Voto response time < 150ms': (r) => r.timings.duration < 150,
    });
  }
  // Cenário 3 (5%): Consulta de Integridade / Redação
  else {
    const res = http.get(`${BASE_URL}/health`);
    check(res, {
      'Health check status 200': (r) => r.status === 200,
    });
  }

  sleep(0.1);
}
