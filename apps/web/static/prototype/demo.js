const DURATION = 102000;

const scenes = [
	{
		id: 'title',
		at: 0,
		step: null,
		title: 'Docufill, end to end',
		body: 'Sign up, save your details once, and let the form fill itself.'
	},
	{
		id: 'signup',
		at: 5000,
		step: 'signup',
		title: '1. Sign up with Google',
		body: 'Name, email and photo come across. The inbox is never touched.'
	},
	{
		id: 'onboarding',
		at: 12500,
		step: 'details',
		title: '2. Answer a few questions once',
		body: 'Three short screens: about you, where you live, work.'
	},
	{
		id: 'vault',
		at: 24000,
		step: 'details',
		title: '2. Saved and reusable',
		body: 'Twelve confirmed details, encrypted. Typed once, reused forever.'
	},
	{
		id: 'upload',
		at: 30000,
		step: 'upload',
		title: '3. Add the real PDF',
		body: 'Tenancy Application.pdf. Named automatically, original never edited.'
	},
	{
		id: 'processing',
		at: 37500,
		step: 'upload',
		title: '3. Fields found, details matched',
		body: 'Docufill reads the page and checks it against what he already saved.'
	},
	{
		id: 'copilot',
		at: 43500,
		step: 'answer',
		title: '4. 18 fields on this page',
		body: 'He only has to answer one of them. Three belong to his guarantor.'
	},
	{
		id: 'questions',
		at: 49000,
		step: 'answer',
		title: '4. Answers land on the document',
		body: 'Saved details write themselves in. He types the rent, and nothing else.'
	},
	{
		id: 'invite',
		at: 64000,
		step: 'invite',
		title: '5. Send the rest to someone else',
		body: 'Email, private link, six-digit code. Adaeze gets only her three questions.'
	},
	{
		id: 'participant',
		at: 72000,
		step: 'invite',
		title: '5. She fills her part without an account',
		body: 'Code, three answers, save. She never sees the PDF.'
	},
	{
		id: 'sign',
		at: 81500,
		step: 'sign',
		title: '6. Review, consent, sign',
		body: 'The completed PDF is rendered from the untouched original.'
	},
	{
		id: 'done',
		at: 90000,
		step: 'done',
		title: '7. Completed PDF',
		body: 'Signed and downloadable. The link expires in five minutes.'
	},
	{
		id: 'reuse',
		at: 96000,
		step: 'done',
		title: 'And the next one is shorter',
		body: 'Same details, already matched. Review instead of retype.'
	}
];

const onboardingSteps = [
	{
		start: 0,
		pill: '1 of 3',
		title: 'About you',
		note: 'Google already gave his name. He adds three more things.',
		cta: 'Continue',
		press: 3950,
		fields: [
			{ label: 'Full name', value: 'Chinedu Okonkwo', tag: 'From Google' },
			{ label: 'Phone number', value: '+234 803 555 0142', from: 400, to: 1900 },
			{ label: 'Date of birth', value: '14 May 1995', from: 2050, to: 2950 },
			{ label: 'Nationality', value: 'Nigerian', from: 3050, to: 3750 }
		]
	},
	{
		start: 4300,
		pill: '2 of 3',
		title: 'Where you live',
		note: 'One address now fills every residence and postal field later.',
		cta: 'Continue',
		press: 7800,
		fields: [
			{ label: 'Home address', value: '27 Ologun Agbaje St, Victoria Island', from: 4500, to: 6300 },
			{ label: 'City', value: 'Lagos', from: 6400, to: 6800 },
			{ label: 'State', value: 'Lagos', from: 6900, to: 7300 },
			{ label: 'Country', value: 'Nigeria', tag: 'Prefilled' }
		]
	},
	{
		start: 8200,
		pill: '3 of 3',
		title: 'Work',
		note: 'Employment shows up on tenancy, HR and school forms alike.',
		cta: 'Save and continue',
		press: 11100,
		fields: [
			{ label: 'Occupation', value: 'Backend Engineer', from: 8400, to: 9300 },
			{ label: 'Current employer', value: 'Bujeti Limited', from: 9400, to: 10300 },
			{ label: 'Employment status', value: 'Full-time employee', from: 10400, to: 10900 }
		]
	}
];

const participantFields = [
	{ label: 'Guarantor full name', value: 'Adaeze Okonkwo', from: 4500, to: 5500 },
	{ label: 'Relationship to applicant', value: 'Sister', from: 5600, to: 6100 },
	{ label: 'Phone number', value: '+234 809 441 2201', from: 6200, to: 7600 }
];

const applicantKeys = ['name', 'email', 'phone', 'address', 'employer', 'rent'];
const guarantorKeys = ['gname', 'grel', 'gphone'];
const allKeys = [...applicantKeys, ...guarantorKeys];

const el = (id) => document.getElementById(id);
const sceneNodes = new Map(
	[...document.querySelectorAll('.scene')].map((node) => [node.dataset.scene, node])
);
const stepNodes = [...document.querySelectorAll('.steps span')];
const papers = new Map();
const paperText = new Map();

for (const slot of document.querySelectorAll('[data-paper]')) {
	const node = el('paper-tpl').content.firstElementChild.cloneNode(true);
	slot.append(node);
	papers.set(slot.dataset.paper, node);
	for (const value of node.querySelectorAll('.val')) {
		paperText.set(value, value.textContent);
	}
}

const capTitle = el('cap-title');
const capBody = el('cap-body');
const progress = el('progress');
const music = el('music');

function typed(value, from, to, time) {
	if (time <= from) return '';
	if (time >= to) return value;
	const ratio = (time - from) / (to - from);
	return value.slice(0, Math.max(1, Math.round(ratio * value.length)));
}

function renderFields(host, fields) {
	host.textContent = '';
	for (const field of fields) {
		const wrap = document.createElement('label');
		wrap.className = 'fld';
		const label = document.createElement('span');
		label.textContent = field.label;
		const box = document.createElement('div');
		box.className = 'box';
		const value = document.createElement('span');
		box.append(value);
		if (field.tag) {
			const tag = document.createElement('span');
			tag.className = 'tag known';
			tag.textContent = field.tag;
			box.append(tag);
		}
		wrap.append(label, box);
		host.append(wrap);
		field.node = box;
		field.valueNode = value;
	}
}

function fillFields(fields, time) {
	for (const field of fields) {
		if (field.from === undefined) {
			field.valueNode.textContent = field.value;
			field.node.classList.add('filled');
			continue;
		}
		const text = typed(field.value, field.from, field.to, time);
		field.valueNode.textContent = text || 'Waiting';
		field.valueNode.classList.toggle('ph', !text);
		field.node.classList.toggle('typing', Boolean(text) && time < field.to);
		field.node.classList.toggle('filled', time >= field.to);
	}
}

function press(node, time, at, hold = 350) {
	node.classList.toggle('press', time >= at && time < at + hold);
}

function writePaper(name, keys, extra = {}) {
	const paper = papers.get(name);
	if (!paper) return;
	for (const value of paper.querySelectorAll('.val')) {
		const key = value.dataset.val;
		const override = extra[key];
		value.textContent = override === undefined ? paperText.get(value) : override;
		value.classList.toggle('in', keys.includes(key) || Boolean(override));
	}
	for (const line of paper.querySelectorAll('.pl')) {
		line.classList.toggle('hit', line.dataset.box === extra.focus);
	}
}

let onboardingIndex = -1;
let participantReady = false;

function apply(time) {
	let index = 0;
	while (index + 1 < scenes.length && time >= scenes[index + 1].at) index += 1;
	const scene = scenes[index];
	const local = time - scene.at;

	for (const [id, node] of sceneNodes) node.classList.toggle('is-on', id === scene.id);
	const stepIndex = stepNodes.findIndex((node) => node.dataset.step === scene.step);
	stepNodes.forEach((node, position) => {
		node.classList.toggle('on', position === stepIndex);
		node.classList.toggle('done', stepIndex >= 0 && position < stepIndex);
	});
	capTitle.textContent = scene.title;
	capBody.textContent = scene.body;
	progress.style.width = `${Math.min(100, (time / DURATION) * 100)}%`;

	if (scene.id === 'signup') {
		const signedIn = local >= 3000;
		press(el('su-btn'), local, 1100, 700);
		el('su-btn-text').textContent = signedIn
			? 'Signed in with Google'
			: local >= 1800
				? 'Opening Google…'
				: 'Continue with Google';
		el('su-title').textContent = signedIn
			? 'Welcome, Chinedu'
			: 'Fill forms once. Never fill them again.';
		el('su-lede').textContent = signedIn
			? 'Your name, email and photo came from Google.'
			: 'Create your account to get started.';
		el('su-card').classList.toggle('show', signedIn);
		el('su-note').classList.toggle('show', local >= 3600);
	}

	if (scene.id === 'onboarding') {
		let step = 0;
		while (step + 1 < onboardingSteps.length && local >= onboardingSteps[step + 1].start) step += 1;
		const current = onboardingSteps[step];
		if (step !== onboardingIndex) {
			onboardingIndex = step;
			renderFields(el('ob-fields'), current.fields);
			el('ob-pill').textContent = current.pill;
			el('ob-title').innerHTML = `<span class="mark">${current.title}</span>`;
			el('ob-cta').textContent = current.cta;
			el('ob-note').textContent = current.note;
		}
		fillFields(current.fields, local);
		press(el('ob-cta'), local, current.press);
	} else {
		onboardingIndex = -1;
	}

	if (scene.id === 'upload') {
		const name = typed('Tenancy Application', 2300, 3900, local);
		el('up-drop').classList.toggle('hot', local >= 1200 && local < 2200);
		el('up-picked').classList.toggle('show', local >= 2200);
		el('up-name').textContent = name;
		el('up-tick').classList.toggle('on', local >= 4300);
		press(el('up-cta'), local, 5200, 500);
		el('up-cta').textContent = local >= 5700 ? 'Uploading…' : 'Upload document';
		el('up-note').textContent =
			local >= 5700
				? 'Confirmed. The original file is stored, never edited.'
				: 'Digital PDFs only · 25 MB max · the original is never edited.';
	}

	if (scene.id === 'processing') {
		const ratio = Math.min(1, local / 5200);
		el('pr-meter').style.width = `${ratio * 100}%`;
		el('pr-pct').textContent = `${Math.round(ratio * 100)}%`;
		const marks = [0, 1200, 2400, 3800];
		for (const row of el('pr-stages').children) {
			const reached = local >= marks[Number(row.dataset.stage)];
			row.classList.toggle('on', reached);
			row.querySelector('.tick').classList.toggle('on', reached);
		}
	}

	if (scene.id === 'copilot') {
		papers.get('copilot').classList.add('boxed');
		papers.get('copilot').querySelector('.paper-meta').textContent =
			'18 fields found on this page';
		writePaper('copilot', []);
	}

	if (scene.id === 'questions') {
		const beats = [
			{ q: 'name', at: 0, write: ['name', 'email'], note: 'His name and email came from Google.' },
			{ q: 'phone', at: 2200, write: ['phone'], note: 'His phone number came from sign up.' },
			{
				q: 'address',
				at: 4200,
				write: ['address'],
				note: 'His address came from sign up. Nothing to type.'
			},
			{
				q: 'employer',
				at: 6300,
				write: ['employer'],
				note: 'His employer came from sign up too.'
			},
			{ q: 'rent', at: 8300, write: [], note: 'Only this one is new. He types it once.' },
			{
				q: 'gname',
				at: 12200,
				write: [],
				note: 'These three are not his to answer. They go to his guarantor.'
			}
		];
		let beat = 0;
		while (beat + 1 < beats.length && local >= beats[beat + 1].at) beat += 1;
		const active = beats[beat];
		const written = beats.slice(0, beat + 1).flatMap((item) => item.write);
		const rent = typed('₦2,400,000', 8900, 11200, local);

		el('qn-note').textContent = active.note;
		el('qn-rent').textContent = rent || 'Type your answer';
		el('qn-rent-tag').textContent = local >= 11200 ? 'You answered' : 'Needs you';
		el('qn-rent-tag').className = `tag ${local >= 11200 ? 'known' : 'ask'}`;
		el('qn-guar').textContent = local >= 12200 ? 'Send to Adaeze' : 'Not yours to answer';
		for (const row of el('qn-rows').children) {
			row.classList.toggle('on', row.dataset.q === active.q);
		}
		writePaper('questions', written, { rent: rent || undefined, focus: active.q });
		papers.get('questions').querySelector('.paper-meta').textContent =
			'Live preview · your answers';
	}

	if (scene.id === 'invite') {
		const mail = typed('adaeze.okonkwo@gmail.com', 800, 3000, local);
		const box = el('iv-mail');
		box.firstElementChild.textContent = mail || 'name@example.com';
		box.firstElementChild.classList.toggle('ph', !mail);
		box.classList.toggle('typing', Boolean(mail) && local < 3000);
		box.classList.toggle('filled', local >= 3000);
		press(el('iv-cta'), local, 3400, 450);
		el('iv-sent').textContent =
			local >= 5200
				? 'Waiting for Adaeze. She does not need an account.'
				: local >= 4000
					? 'Email sent · private link ready · six-digit code in her inbox'
					: '';
	}

	if (scene.id === 'participant') {
		const verifying = local >= 1900 && local < 4300;
		const answering = local >= 4300;
		const code = typed('408231', 2100, 3600, local);
		el('pt-title').textContent = answering
			? 'Your three questions'
			: verifying
				? 'Check your email'
				: 'Chinedu asked you to complete the guarantor section';
		el('pt-lede').textContent = answering
			? 'You will only see these. Not the PDF, not his answers.'
			: verifying
				? 'Enter the six-digit code from your invitation.'
				: 'You will only see the questions assigned to you — not the PDF.';
		el('pt-code').style.display = verifying ? 'flex' : 'none';
		[...el('pt-code').children].forEach((slot, position) => {
			slot.textContent = code[position] ?? '';
		});
		if (answering && !participantReady) {
			renderFields(el('pt-fields'), participantFields);
			participantReady = true;
		}
		if (!answering && participantReady) {
			el('pt-fields').textContent = '';
			participantReady = false;
		}
		if (answering) fillFields(participantFields, local);
		el('pt-cta').textContent = answering
			? 'Save my answers'
			: verifying
				? 'Continue securely'
				: 'Verify and continue';
		press(el('pt-cta'), local, answering ? 8000 : verifying ? 4000 : 1500, 420);
		el('pt-receipt').classList.toggle('show', local >= 8600);
		el('pt-note').textContent =
			local >= 8600
				? 'Her answers are locked onto the document. No account, no app, no password.'
				: "Her answers lock onto Chinedu's document. That is all she has to do.";
	} else {
		participantReady = false;
	}

	if (scene.id === 'sign') {
		writePaper('sign', allKeys);
		papers.get('sign').querySelector('.paper-meta').textContent =
			local >= 4200 ? 'Signed · every answer on the page' : 'Completed preview · review before signing';
		papers.get('sign').querySelector('.sig').classList.toggle('draw', local >= 4200);
		el('sg-tick').classList.toggle('on', local >= 2600);
		press(el('sg-cta'), local, 3600, 450);
		el('sg-cta').textContent = local >= 4100 ? 'Signed' : 'Sign and complete';
	}

	if (scene.id === 'done') {
		writePaper('done', allKeys);
		papers.get('done').querySelector('.paper-meta').textContent =
			'Signed · Tenancy Application.pdf';
		papers.get('done').querySelector('.sig').classList.add('done');
		press(el('dn-cta'), local, 2600, 500);
		el('dn-cta').textContent = local >= 3100 ? 'Downloaded' : 'Download completed PDF';
	}
}

let started = 0;
let offset = 0;
let playing = true;

function now() {
	return playing ? performance.now() - started : offset;
}

function frame() {
	const time = now();
	if (time >= DURATION) {
		offset = DURATION;
		playing = false;
		el('toggle').textContent = 'Replay';
		apply(DURATION - 1);
		syncMusic();
		return;
	}
	apply(time);
	syncMusic();
	requestAnimationFrame(frame);
}

function syncMusic() {
	if (!playing || music.muted) {
		if (!music.paused) music.pause();
		return;
	}
	const time = now() / 1000;
	if (music.paused) void music.play().catch(() => {});
	if (Math.abs(music.currentTime - time) > 0.4) music.currentTime = time;
}

function seek(time) {
	started = performance.now() - time;
	offset = time;
	apply(time);
}

el('toggle').addEventListener('click', () => {
	if (!playing && offset >= DURATION) {
		seek(0);
		playing = true;
		el('toggle').textContent = 'Pause';
		requestAnimationFrame(frame);
		return;
	}
	playing = !playing;
	if (playing) {
		started = performance.now() - offset;
		el('toggle').textContent = 'Pause';
		requestAnimationFrame(frame);
	} else {
		offset = now();
		el('toggle').textContent = 'Play';
	}
});

el('mute').addEventListener('click', () => {
	music.muted = !music.muted;
	el('mute').textContent = music.muted ? 'Unmute' : 'Mute';
});

el('restart').addEventListener('click', () => {
	seek(0);
	playing = true;
	el('toggle').textContent = 'Pause';
	music.currentTime = 0;
	requestAnimationFrame(frame);
});

const params = new URLSearchParams(location.search);
if (params.has('record')) document.body.classList.add('record');
if (params.has('mute')) music.muted = true;

const at = Number(params.get('at'));
if (Number.isFinite(at) && at > 0) {
	playing = false;
	offset = at;
	apply(at);
	el('toggle').textContent = 'Play';
} else {
	started = performance.now();
	requestAnimationFrame(frame);
}
