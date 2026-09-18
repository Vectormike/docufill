/// <reference lib="webworker" />

import { build, files, version } from '$service-worker';

const worker = self as unknown as ServiceWorkerGlobalScope;
const CACHE = `docufill-shell-${version}`;
const PUBLIC_ASSETS = [...build, ...files].filter(
	(path) => !path.endsWith('.pdf') && !path.includes('signature')
);
const SENSITIVE_PATHS = [
	'/profile',
	'/documents',
	'/settings',
	'/onboarding',
	'/share',
	'/auth',
	'/api',
	'/v1'
];

worker.addEventListener('install', (event) => {
	event.waitUntil(
		caches.open(CACHE).then((cache) => cache.addAll([...PUBLIC_ASSETS, '/', '/offline']))
	);
});

worker.addEventListener('message', (event) => {
	if (event.data?.type === 'SKIP_WAITING') {
		void worker.skipWaiting();
	}
});

worker.addEventListener('activate', (event) => {
	event.waitUntil(
		caches
			.keys()
			.then((keys) =>
				Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key)))
			)
			.then(() => worker.clients.claim())
	);
});

worker.addEventListener('fetch', (event) => {
	const { request } = event;
	const url = new URL(request.url);
	if (request.method !== 'GET' || url.origin !== worker.location.origin) return;

	const hasCredentials =
		request.headers.has('authorization') ||
		request.headers.has('x-participant-session') ||
		SENSITIVE_PATHS.some((path) => url.pathname.startsWith(path));
	if (hasCredentials) {
		event.respondWith(
			fetch(request).catch(async () => {
				if (request.mode === 'navigate') {
					return (await caches.match('/offline')) ?? Response.error();
				}
				return Response.error();
			})
		);
		return;
	}

	if (PUBLIC_ASSETS.includes(url.pathname)) {
		event.respondWith(caches.match(request).then((cached) => cached ?? fetch(request)));
		return;
	}

	if (request.mode === 'navigate') {
		event.respondWith(
			fetch(request).catch(async () => (await caches.match('/offline')) ?? Response.error())
		);
	}
});
