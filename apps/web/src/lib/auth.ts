import type { Session } from '@supabase/supabase-js';
import { getSupabase } from './supabase';

export async function currentSession(): Promise<Session | null> {
	const { data, error } = await getSupabase().auth.getSession();
	if (error) throw error;
	return data.session;
}

export async function signInWithGoogle(): Promise<void> {
	const { error } = await getSupabase().auth.signInWithOAuth({
		provider: 'google',
		options: {
			redirectTo: `${window.location.origin}/auth/callback`
		}
	});
	if (error) throw error;
}

export async function signInWithEmail(email: string): Promise<void> {
	const { error } = await getSupabase().auth.signInWithOtp({
		email,
		options: {
			emailRedirectTo: `${window.location.origin}/auth/callback`
		}
	});
	if (error) throw error;
}

export async function signOut(): Promise<void> {
	const { error } = await getSupabase().auth.signOut();
	if (error) throw error;
}
