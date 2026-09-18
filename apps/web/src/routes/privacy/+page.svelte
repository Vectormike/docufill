<script lang="ts">
	import { env } from '$env/dynamic/public';
	import LegalHeader from '$lib/components/LegalHeader.svelte';

	const privacyEmail = env.PUBLIC_PRIVACY_EMAIL || 'privacy@docufill.app';
</script>

<svelte:head>
	<title>Privacy notice — Docufill</title>
	<meta
		name="description"
		content="How Docufill handles personal information for document completion."
	/>
</svelte:head>

<LegalHeader />
<main class="prose-legal mx-auto max-w-4xl px-5 py-10 sm:px-8 sm:py-14">
	<p class="eyebrow mb-2">Nigeria</p>
	<h1>Privacy notice</h1>
	<p class="lead">
		Effective 15 September 2026. This notice explains how Docufill handles information when you
		create a reusable profile, complete documents, sign, or participate in another person's request.
	</p>

	<h2>What we collect</h2>
	<p>
		We process account details, details you choose to share after sign-in or later in your profile,
		uploaded documents, answers, signatures, participant contact details, consent records, and
		security/audit events. We derive limited facts—such as employment duration—only from confirmed
		information.
	</p>

	<h2>Why we use it</h2>
	<p>
		We use this information to authenticate you, understand and complete your documents, coordinate
		assigned participant sections, generate previews and downloads, prevent abuse, and honour your
		privacy requests. Optional document memory is processed only after you enable it for that
		document.
	</p>

	<h2>AI and document memory</h2>
	<p>
		Deterministic matches are preferred. AI receives only context relevant to a field and its
		suggestions remain drafts until you confirm them. Identity and financial details are never
		invented. Customer documents are not used to train AI models. Optional semantic memory excludes
		facts marked sensitive and can be disabled, which queues deletion of the related index.
	</p>

	<h2>Sharing and processors</h2>
	<p>
		We share information only with service providers needed to operate Docufill, such as hosting,
		encrypted storage, authentication, email delivery, and configured AI processing. A participant
		can access only their assigned questions and the document subject after verification. They
		cannot view the PDF or another participant's information.
	</p>

	<h2>Retention and deletion</h2>
	<p>
		Documents, profile facts, saved signatures, and audit events remain until you delete the
		applicable data or your account. Participant links expire after seven days, verification codes
		after fifteen minutes, verified sessions after two hours, and download links after five minutes.
		Operational backups may persist for the provider's limited backup cycle before being
		overwritten.
	</p>

	<h2>Security</h2>
	<p>
		Docufill uses encryption in transit, application-level encryption for reusable facts and
		signatures, private storage, short-lived signed URLs, scoped access policies, request limits,
		recent-authentication checks for signature and account operations, and append-only audit
		controls. No internet service can promise absolute security.
	</p>

	<h2>Your rights</h2>
	<p>
		Subject to applicable Nigerian data protection law, you may ask to access, export, correct,
		delete, restrict, or object to processing of your information, and may withdraw optional
		consent. Account settings provide export, correction, memory controls, and deletion. You may
		also complain to the Nigeria Data Protection Commission.
	</p>

	<h2>Contact</h2>
	<p>
		For a privacy request or question, email <a href={`mailto:${privacyEmail}`}>{privacyEmail}</a>.
		We may verify your identity before fulfilling a request.
	</p>
</main>
