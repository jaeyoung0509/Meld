import http from 'k6/http';
import { check, sleep } from 'k6';

const vus = Number(__ENV.K6_REST_VUS || 20);
const duration = __ENV.K6_REST_DURATION || '12s';
const p95Ms = Number(__ENV.K6_REST_P95_MS || 120);
const errRate = Number(__ENV.K6_REST_ERR_RATE || 0.01);

export const options = {
  vus,
  duration,
  thresholds: {
    http_req_failed: [`rate<=${errRate}`],
    http_req_duration: [`p(95)<=${p95Ms}`],
  },
};

export default function () {
  const baseUrl = __ENV.K6_REST_BASE_URL || 'http://127.0.0.1:3000';
  const path = __ENV.K6_REST_PATH || '/health';

  const response = http.get(`${baseUrl}${path}`);
  check(response, {
    'status is 200': (r) => r.status === 200,
  });

  sleep(0.1);
}
