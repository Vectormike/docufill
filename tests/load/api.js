import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
	scenarios: {
		owner_reads: {
			executor: 'ramping-vus',
			startVUs: 0,
			stages: [
				{ duration: '20s', target: 10 },
				{ duration: '40s', target: 10 },
				{ duration: '20s', target: 0 }
			]
		}
	},
	thresholds: {
		http_req_failed: ['rate<0.01'],
		'http_req_duration{endpoint:health}': ['p(95)<250'],
		'http_req_duration{endpoint:document}': ['p(95)<750']
	}
};

const baseUrl = __ENV.BASE_URL || 'http://127.0.0.1:8080';

export default function () {
	const health = http.get(`${baseUrl}/health`, { tags: { endpoint: 'health' } });
	check(health, { 'health is ready': (response) => response.status === 200 });

	if (__ENV.ACCESS_TOKEN && __ENV.DOCUMENT_ID) {
		const document = http.get(`${baseUrl}/v1/documents/${__ENV.DOCUMENT_ID}`, {
			headers: { Authorization: `Bearer ${__ENV.ACCESS_TOKEN}` },
			tags: { endpoint: 'document' }
		});
		check(document, { 'document is readable': (response) => response.status === 200 });
	}
	sleep(1);
}

