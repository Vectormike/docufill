<script lang="ts">
	import LegalHeader from '$lib/components/LegalHeader.svelte';
</script>

<svelte:head>
	<title>Supported and excluded documents — Docufill</title>
	<meta name="description" content="Documents that Docufill does and does not support." />
</svelte:head>

<LegalHeader />
<main class="prose-legal mx-auto max-w-4xl px-5 py-10 sm:px-8 sm:py-14">
	<p class="eyebrow mb-2">Completion-first scope</p>
	<h1>Supported and excluded documents</h1>
	<p class="lead">
		Docufill helps people complete routine, low-risk administrative PDFs. It is not legal advice, a
		notary, an identity-verification authority, or a qualified electronic-signature service.
	</p>

	<h2>Supported at launch</h2>
	<p>
		Initial support covers digitally generated or fillable PDFs for routine employment, rental,
		school, vendor, and general onboarding workflows. You remain responsible for reviewing every
		answer and the actual rendered PDF before completing or signing it.
	</p>

	<h2>Not supported</h2>
	<ul>
		<li>
			Wills, testamentary documents, powers of attorney, affidavits, deeds, or documents requiring
			notarisation, witnesses, or official seals.
		</li>
		<li>
			Court filings, sworn statements, land transfers, or documents that create or transfer a major
			legal interest.
		</li>
		<li>
			Credit agreements, loan guarantees, insurance underwriting, investment instructions, or
			high-value financial authorisations.
		</li>
		<li>
			Government identity issuance, immigration or visa declarations, tax filings, regulated KYC
			decisions, or documents prohibited from electronic execution.
		</li>
		<li>
			Any document where an issuing organisation forbids automated completion or electronic
			signatures.
		</li>
	</ul>

	<h2>Scanned and handwritten forms</h2>
	<p>
		Printed scans of forms are supported and still require your review before anything is completed.
		Handwriting recognition is not supported in this MVP. Password-protected, corrupted, or
		unsupported PDFs are rejected.
	</p>

	<h2>Signatures and authority</h2>
	<p>
		A stored signature is applied only after recent authentication, review of the rendered PDF, and
		explicit per-document confirmation. Never upload or apply another person's signature.
		Participants create their own isolated signature for their assigned request.
	</p>

	<h2>Your responsibility</h2>
	<p>
		Confirm that electronic completion and signing are accepted by the recipient and appropriate for
		the document. If a document may affect important legal, financial, property, immigration, tax,
		or health rights, use the issuer's approved process and seek qualified advice.
	</p>
</main>
