import { browser } from '$app/environment';
import { env } from '$env/dynamic/public';
import { createBrowserClient } from '@supabase/ssr';
import type { SupabaseClient } from '@supabase/supabase-js';

let client: SupabaseClient | undefined;

export function isSupabaseConfigured(): boolean {
	return Boolean(env.PUBLIC_SUPABASE_URL && env.PUBLIC_SUPABASE_ANON_KEY);
}

export function getSupabase(): SupabaseClient {
	const url = env.PUBLIC_SUPABASE_URL;
	const key = env.PUBLIC_SUPABASE_ANON_KEY;
	if (!browser || !url || !key) {
		throw new Error('Supabase is not configured. Add the public values from .env.example.');
	}
	client ??= createBrowserClient(url, key, {
		auth: {
			detectSessionInUrl: false,
			experimental: {
				appendPkceFlowIdToRedirects: true
			}
		}
	});
	return client;
}

export function apiBaseUrl(): string {
	return (env.PUBLIC_API_URL || 'http://localhost:8080').replace(/\/$/, '');
}
