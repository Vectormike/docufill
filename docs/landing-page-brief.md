# Landing page brief

The current homepage is a **sign-in first** pitch. A remove.bg-style page inverts that: the product is the hero, the aha happens before an account, and sign-in is the reward after a filled form.

The aha is not “upload, then maybe reuse later.” It is: **your Gmail profile lands on a blank form first**, you only finish the gaps, and the next form of that kind is already shorter.

Today Docufill only fills **digital PDFs**. Web-application filling is not in the product yet. On the landing page, treat it as a second try-mode: an in-page sample web form that fills from a demo profile, not a live “paste any URL / browser extension” claim.

Gmail here means **Google account profile** (name, email, photo) via Continue with Google. It does not mean reading the inbox.

---

## Positioning

**One-liner:** Start from your Gmail profile. Every next form takes less time.

**Job:** Someone in Nigeria has another employment, rental, school, vendor, or onboarding form. They connect Google, see name and email already on the page, answer only the gaps, send the rest (guarantor, co-applicant, next of kin) by email and a private link, and leave with a completed document. The invited person does not need a Docufill account.

**Promise to prove in the first 20 seconds:** A blank form receives a Gmail profile, becomes a short checklist, most fields already filled, with a source on every answer.

Keep the existing line: *Fill forms once. Never fill them again.* Explain it as compounding: the more you use Docufill, the less you type on the next one.

---

## Page map

1. **Hero + Try it** — Gmail profile → form fills, above the fold
2. **How it works** — connect Gmail → seed the form → answer gaps → invite the rest by email + link → save into a pack → review → sign
3. **It gets faster** — each confirmed fill shortens the next one
4. **Packs by use case** — tenancy, employment, school, vendor; same person, different forms
5. **Invite without an account** — email + private link; they fill only their part and save
6. **What it fills** — PDFs and web applications, from the same packs
7. **Signatures**
8. **Privacy** — Google profile, not inbox; invitees never see the PDF; private by default
9. **Scope** — supported vs excluded
10. **Sign in** — Google first (this is the real Gmail-profile step), or email magic link
11. **Footer** — privacy, acceptable use, install later (PWA)

Do not lead with the feature carousel. That becomes supporting proof after the try section.

---

## Hero: the remove.bg pattern

remove.bg works because the tool *is* the landing page: a giant drop zone, sample files if you have nothing, instant before/after, then download/sign-up.

For Docufill, the before/after is not a photo slider. It is:

| Before | After |
|---|---|
| Blank PDF page or empty web form | Same page with Gmail name/email already written in |
| Dense fields | Short checklist: from Gmail / from your tenancy pack / needs you / sent to someone else |

### Layout

- Left: headline, one sentence, two trust chips (*Starts from your Gmail profile*, *Private by default*).
- Right (dominant): **Try Docufill** card with two tabs.

**Tab A — Upload a PDF**
**Tab B — Fill a web application**

No account required for the demo. The demo still *shows* a Gmail-profile pull: a synthetic Google card (for example Chinedu Okonkwo) seeds the sample form, then remaining fields fill from a sample pack. Guest files are ephemeral and not saved unless they sign in.

### The Gmail-first beat (every try)

Before fields write onto the page, show a short, visible step:

1. **Connect Gmail** — demo button labelled *Use a sample Gmail profile* (not a live OAuth popup).
2. **Profile card** — avatar, full name, email. Copy: *These details come from Google. We do not read your inbox.*
3. **Fill** — name and email land first, then pack fields (address, employer, next of kin) according to the sample type.
4. **Gaps** — whatever Gmail and the pack do not know stays *needs you*, or *send to someone else*.

That sequence is the landing-page story. Do not skip straight to a fully filled form. After the fill, one assigned line (guarantor name, next of kin) should show the invite beat: an email field, a private link, *They don’t need an account.*

### Tab A: PDF try (core)

Giant dashed drop zone, same energy as `/documents/new`:

- Drop or tap · PDF only · 25 MB max
- Warning: scans and image-only PDFs are not supported
- Row of **sample files** so nobody has to hunt a PDF. Each sample is a **use-case pack**:
  - Tenancy application
  - Employment onboarding
  - School / vendor form

Use synthetic fixtures only. Never real personal data.

**States**

1. **Idle** — drop zone + samples
2. **Accepted** — filename, size, auto-name (same as `subjectFromFilename`), inferred pack (tenancy / employment / school)
3. **Gmail seed** — sample Google profile card, then name/email write onto the page
4. **Processing** — progress copy that matches the product: *Checking PDF → Uploading → Finding fields → Applying your Gmail profile → Grounding the rest from your pack*
5. **Result** — split view
   - Left: original page with yellow field boxes
   - Right: checklist with source badges (`Gmail profile`, `your tenancy pack` / `your employment pack`, `draft — review`, `needs you`, `sent to Ada — email + link`)
   - Copilot strip: *I found N fields · X from Gmail · Y from your pack · Z need you · 1 sent to a guarantor*
6. **Filled preview** — answers write onto the page (the remove.bg “result image”)
7. **Conversion** — *Continue with Google to fill from your profile, keep this pack, invite a guarantor by email, and download the completed PDF*
8. **Reject** — encrypted, scan-only, oversized, malformed, or excluded/high-risk. Point to acceptable use. Offer another sample.

Optional guest download: watermarked or time-limited preview only. Final signed PDF stays behind recent auth + consent, as the product already requires.

### Tab B: web application try

Do not ask people to paste a live job-portal URL. That overclaims.

Ship an **in-page browser chrome** with a sample application:

- School admission
- Job / HR onboarding
- Vendor / KYC-lite (non-regulated)

Same Gmail-first beat: sample Google profile, then **Fill this application**. Name and email populate first, then pack fields (phone, address, employer). Empty fields stay highlighted as *needs you*.

That is the web-form analog of remove.bg’s sample images: one click, visible fill, no setup.

Label it clearly: **Try a sample web application**. If a real extension or “fill this tab” feature ships later, this demo stays the landing proof.

---

## Functionality to show

Use these as sections and as the story the try-tool just demonstrated.

### 1. Start from Gmail

Continue with Google. Name, email, and photo from the Google account seed the first form immediately. Optional onboarding (phone, address, work) only asks what Gmail cannot provide.

Do not claim inbox access, attachment import, or scanning old mail.

### 2. Upload a digital PDF

Direct upload to private storage. Original never modified. Auto-named. Optional pasted context. User must confirm it is a supported, non-regulated document. The first fields to fill are the Gmail-seeded identity fields.

### 3. Document Copilot

Turns the PDF into fields, then a short checklist. Counts: from Gmail, from the matching pack, missing questions, AI drafts, signature fields, participant sections.

### 4. It gets faster every time

The first form still has gaps. Each confirmed answer is kept. The second tenancy form should be mostly pack + Gmail. The tenth should be a review, not a rewrite.

Show this as a simple progression, not a feature list:

- Form 1 — Gmail fills name and email; you add address, landlord details, next of kin
- Form 2 — those answers return; you only handle what is new
- Form 3 — you mostly review and sign

### 5. Packs by use case

One person, several lives of paperwork. Group confirmed details into packs so a job form does not drag in landlord facts, and a tenancy form does not dump employer history unless the PDF asks for it.

Suggested packs on the page (Nigeria-first, matching the sample files):

- **Tenancy** — identity, contact, address, next of kin, guarantor-ready fields
- **Employment** — identity, contact, work history, education
- **School** — identity, contact, education, guardian
- **Vendor / onboarding** — identity, contact, business basics

The matching pack is chosen from the document type. Users can open a pack, edit it, or reuse it on the next form of that kind.

Confirmed facts only. Sensitive values stay encrypted; AI never invents identity or money.

### 6. Answer questions, not pages

Source on every line (`Gmail profile`, pack name, `needs you`, `sent to Ada`). User can edit, reject a draft, or assign a field. Any change invalidates the preview.

### 7. Fill web applications (landing surface)

Same Gmail seed and same packs, HTML form instead of PDF. Demo on the landing page; full product would later reuse this mapping. Do not promise autofill of arbitrary third-party sites until that exists.

### 8. Invite by email and a link — they do not need an account

Tenancy, employment, and school forms almost always need someone else: a guarantor, co-applicant, or next of kin. That person should not have to join Docufill to help.

**Owner side**

- Pick the questions that are not yours.
- Invite by **email**. They also get a **private link** (owner can copy it and send on WhatsApp if needed).
- A six-digit code goes to that same inbox so only the invited person can open the questions.
- Owner stays in control of the document, preview, and signature. Assigned fields cannot be answered by the owner until they are reclaimed.

**Invitee side — no user account**

They are invited onto the platform for this document only. Flow:

1. Open the email (or the link).
2. Land on a private participant workspace: *Ada asked you to complete the guarantor section.*
3. Enter the code from the email. No Google sign-in, no password, no app install.
4. See **only their questions** — not the PDF, not the owner’s pack, not anyone else’s answers.
5. Fill their part.
6. **Save.** Their answers lock onto the document so the owner can finish. They get a short receipt, not a login.

**They can save for themselves too.** After they save their part, offer an optional *Keep these answers for next time* (Continue with Google or email link). If they skip it, the owner’s form is still complete. If they accept, the next time they are invited — or the first time they fill their own tenancy form — those confirmed details are already there. Do not force an account to help someone else.

Show this on the page as a two-panel story: owner checklist with *Send to guarantor*, then the invitee’s phone with four questions and a Save button. Copy: *Email + a private link. They never have to become a user.*

Do not claim they can browse other documents, see the filled PDF, or keep access after the link expires.

### 9. Preview, adjust, sign, download

Render from the immutable original. Layout adjuster. Signature applied only after recent login, preview review, and per-document consent. Download links expire in five minutes.

### 10. Privacy controls

Export JSON, delete a document (original + previews + memory), delete account, optional per-document memory of non-sensitive confirmed answers. PWA install is a later convenience, not the hero.

Google is used for sign-in and profile seeding. State that plainly next to the try card and again in Privacy.

---

## Conversion

remove.bg lets you use the tool, then asks you to sign up for more. Mirror that.

After a successful fill:

> This demo used a sample Gmail profile. Continue with Google to fill from *your* name and email, keep a tenancy or employment pack, invite a guarantor by email and a private link, and download a completed PDF. They do not need an account.

Lead with **Continue with Google**. Email magic link is the fallback; it seeds email only, not the Google photo/name card.

Repeat the privacy + acceptable-use line under the form. One extra clause: *We use your Google profile to start the form. We do not read your Gmail inbox.*

Signed-in users skip the demo and go to **My documents**.

---

## Copy and constraints

**Do claim**

- Google / Gmail profile (name, email, photo) seeds the first form
- Each confirmed fill makes the next one shorter
- Packs per use case: tenancy, employment, school, vendor / onboarding
- Digital / fillable PDFs for those routine workflows
- Guided questions, confirmed reuse, explicit signing
- Invite by email and a private link; invitee does not need a Docufill account
- Invitee fills only their questions, saves onto the document, and can optionally keep those answers for next time
- Guest try with sample Gmail profile, sample files, a sample web form, and a sample guarantor invite

**Do not claim**

- Reading the Gmail inbox, importing mail, or attaching files from Google Drive
- Filling any live website, scans/handwriting, legal advice, QES/notary, wills, court, tax, immigration, loans
- “AI fills everything” — drafts stay unconfirmed
- Permanent storage of guest uploads
- That email-magic-link users get the same Gmail-profile card as Google sign-in
- That an invitee can see the PDF, the owner’s pack, or another person’s answers
- That helping on someone else’s form requires creating an account

**Legal on the try card**

One line: *Demo files and the sample Gmail profile are synthetic. Don’t upload wills, court papers, or anything that needs a regulated signature.* Link to `/acceptable-use` and `/privacy`.

---

## Implementation notes

- Guest try needs a **public, ephemeral** extract/map/preview path. Current `uploadDocument` requires a signed-in owner. Do not route landing uploads into real owner storage.
- The demo Gmail step is a **scripted profile card**, not live OAuth. Real Google profile seeding happens after Continue with Google (today: display name, email, avatar from Supabase; not inbox).
- Cap guest size/pages the same as prod (25 MB, 100 pages). Reclaim artifacts quickly.
- Prefer samples over user uploads if guest processing is not ready; samples still give the remove.bg “click and see” moment, as long as the Gmail-seed beat still plays.
- Mobile: drop zone first, headline stacked above. Samples as a horizontal scroller. Result stacks: Gmail card, form page, then checklist.
- Keep light-only, Nigeria-first examples (tenancy, HR, school), and the existing brand (Newsreader + marker yellow).
- Packs on the landing page can be presented as named groups even if the product still stores a single vault of namespaced facts. The story is use-case grouping; the implementation can map tenancy/employment/school onto existing namespaces until dedicated packs ship.
- Invite demo can be scripted (sample email to Ada, sample `/share/…` workspace, Save). Real invites stay on the authenticated document flow: email + private link + 6-digit code, scoped session, no account.
- “Save for themselves” after an invite is optional. Today a participant submits without a vault. On the landing page, still show Save on the document as the required end of their job; keeping answers for next time is an optional Continue with Google on the receipt, until participant packs exist.

The current `apps/web/src/routes/+page.svelte` can stay as the signed-out shell, but the **Try it** card should replace the feature carousel as the first interaction. The carousel copy (vault, questions, invite) becomes the “how it works” section after someone has already seen a Gmail profile land on a form, and a guarantor save their part without signing up.
