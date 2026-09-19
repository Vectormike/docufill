export function isExtraPartyLabel(label: string): boolean {
	const normalized = label
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, ' ')
		.trim();
	if (normalized.includes('additional signatory') || normalized.includes('other signator')) {
		return true;
	}
	const last = normalized.split(/\s+/).at(-1);
	if (last && /^\d+$/.test(last) && last !== '1') return true;
	return /\bsignatory [bcd]\b/.test(normalized);
}
