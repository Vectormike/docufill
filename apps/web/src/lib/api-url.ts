export function resolveApiBaseUrl(configured: string, pageHost?: string): string {
	const trimmed = configured.replace(/\/$/, '');
	if (!pageHost) return trimmed;
	try {
		const api = new URL(trimmed);
		if (isLoopbackHost(api.hostname) && isLoopbackHost(pageHost) && api.hostname !== pageHost) {
			api.hostname = pageHost;
		}
		return api.origin + (api.pathname === '/' ? '' : api.pathname.replace(/\/$/, ''));
	} catch {
		return trimmed;
	}
}

function isLoopbackHost(host: string) {
	return host === 'localhost' || host === '127.0.0.1';
}
