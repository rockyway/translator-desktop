# Replace unofficial Google Translate with official APIs

## Context

Investigation confirmed the app's translate feature was hitting HTTP 429 because
`src-tauri/src/commands/translate.rs` calls the **unofficial** `translate.google.com/translate_a/single?client=gtx`
endpoint (no API key — the same reverse-engineered trick many free translator apps use). A direct `curl` to that
endpoint from this machine reproduced the 429 with Google's own "unusual traffic" block page, confirming this
network's IP has been rate-limited by Google's anti-scraping system — not a Google outage, and not something
retry/backoff can reliably fix long-term.

Decision made with the user: move translation to the **official Google Cloud Translation API (v2 Basic)**, which
requires a paid API key (Google gives 500,000 free characters/month, then $20/million chars) but isn't subject to
this scraping block. That API only returns translated text — it drops the dictionary-style "Additional Information"
panel (definitions/examples/synonyms/alternatives/related words/transliteration) the app currently shows via
`MetadataSection.tsx`. Researched replacements; settled on **freedictionaryapi.com** (free, no key, 1000 req/hr/IP,
multi-language via Wiktionary — verified directly with `curl` for both English and Vietnamese) to repopulate most of
that panel. It has no audio field, but the app already has an independent TTS pipeline (`speak` Tauri command →
Google's `translate_tts` endpoint) reused for a speaker button — the user opted to replace *that* with the
browser's built-in Web Speech API (`SpeechSynthesis`) instead, since WebView2 is Chromium-based, it's free/offline,
and it removes another unofficial-Google-endpoint dependency. `pronunciations[0].text` (IPA) from the dictionary API
slots into the existing "Pronunciation/Transliteration" UI element (`TranslationPanel.tsx:428`) — a good semantic
fit already wired up.

Net effect: "Alternatives" and "Related Words" panels lose their only data source (no free equivalent exists) and
will simply stay empty/hidden — `MetadataSection.tsx` already conditionally hides empty sections, so no UI change
needed for that.

## Backend (Rust)

**`src-tauri/src/commands/translate.rs`** — rewrite `translate()`:
- Add `db_state: State<'_, DbState>` param (same `DbState` used by `settings.rs`, imported via `super::DbState`).
- Look up the API key from `config_store` (`SELECT value FROM config_store WHERE key = 'google_translate_api_key'`),
  same lookup style already used in `lib.rs` for `hotkey_modifier`/`selection_modifier`. If missing, return a new
  `TranslateError::MissingApiKey` with a message pointing the user at Settings — surfaces through the existing
  error banner in `TranslationPanel.tsx`.
- Call `POST https://translation.googleapis.com/language/translate/v2?key={key}` with form body `q`, `target`,
  `format=text`, and `source` only when `from != "auto"` (omitting `source` triggers Google's own auto-detect).
- Parse `{"data":{"translations":[{"translatedText","detectedSourceLanguage"}]}}`.
- On a non-2xx response, parse Google's `{"error":{"message","status"}}` body so quota/auth errors are readable
  instead of a bare status code.
- Delete the `dt=`-specific parser functions (`parse_examples`, `parse_definitions`, `parse_alternatives`,
  `parse_synonyms`, `parse_related_words`, `parse_transliteration`) and the metadata-building block — `translate()`
  now always returns `metadata: None`; metadata comes from the new command below. Update/remove the unit tests tied
  to the deleted parsers.

**New `src-tauri/src/commands/dictionary.rs`**:
- `#[tauri::command] async fn get_dictionary_metadata(word: String, language: String, http_client: State<'_, HttpClientState>) -> Result<Option<TranslationMetadata>, DictionaryError>` — reuses the existing shared `HttpClientState`.
- `GET https://freedictionaryapi.com/api/v1/entries/{language}/{urlencoded word}`.
- 404 (word/language not found) → `Ok(None)`, not an error (panel just won't render, matching current behavior).
- Map response → existing `TranslationMetadata` shape: `entries[].partOfSpeech` + `senses[].definition`/first
  `example` → `Definition`/`DefinitionEntry`; first ~5 `senses[].examples` → `Example`; first ~10
  `senses[].synonyms` → `Synonym`; first entry's first `pronunciations[].text` → `transliteration`. Leave
  `alternatives`/`relatedWords` as empty vectors (no source for them anymore).
- Register `mod dictionary;` / `pub use dictionary::get_dictionary_metadata;` in `commands/mod.rs`, and add the
  import + `tauri::generate_handler!` entry in `lib.rs`.

**Remove `speak.rs`**: delete `src-tauri/src/commands/speak.rs`, its `mod speak;` / `pub use speak::speak;` in
`commands/mod.rs`, and its import + handler registration in `lib.rs`. TTS moves entirely to the frontend (Web
Speech API); `HttpClientState` stays since `translate` and the new dictionary command still use it.

No `Cargo.toml` changes — `reqwest` (with `json`), `urlencoding`, and `serde_json` are already present and cover
both new HTTP calls.

## Frontend

**`src/services/translationService.ts`**:
- `translateText()`: after the core `invoke('translate', ...)` call, if the trimmed source text is short (≤3
  words — matches the old dictionary panel's effective single-word/short-phrase behavior), call a new
  `getDictionaryMetadata(word, lang)` wrapping `invoke('get_dictionary_metadata', ...)` against the
  detected/source language, and attach its result as `metadata`. Swallow dictionary-lookup failures (log + ignore)
  so they never break the main translation result — same pattern already used for the `add_history` failure swallow
  in `useTranslation.ts:119-122`.
- Replace `playTextToSpeech()`: build a `SpeechSynthesisUtterance(text)`, set `utterance.lang = languageCode`, call
  `window.speechSynthesis.speak(utterance)`. Return a small adapter (new exported type `SpeechHandle`) exposing just
  `addEventListener('ended' | 'error', cb)` (wired to `utterance.onend`/`onerror`) and `pause()` + settable
  `currentTime` (wired to `speechSynthesis.cancel()`) — the exact subset of `HTMLAudioElement` all 6 call sites
  already use, so none of their call patterns change. Drop the 200-char truncation (no longer needed) and the
  base64/Blob/`Audio()` plumbing.

**`TranslationPanel.tsx`, `HistoryPanel.tsx`, `PopupOverlay.tsx`** (2 refs each, 6 total): change
`useRef<HTMLAudioElement | null>` → `useRef<SpeechHandle | null>` and add `SpeechHandle` to the existing
`playTextToSpeech` import. No other changes — `.pause()` / `.currentTime = 0` / `.addEventListener(...)` call sites
are unaffected.

**`src/features/settings/SettingsPanel.tsx`**: add a "Translation API" section with a masked input to paste/save
the Google Cloud Translation API key, using the existing generic `invoke('get_setting'|'set_setting', { key:
'google_translate_api_key', ... })` commands (same ones `settings.rs` already exposes) — load current value on
mount, save on submit/blur with a brief confirmation.

**CSP (`src-tauri/tauri.conf.json`)**: no change needed. Both new endpoints (`translation.googleapis.com`,
`freedictionaryapi.com`) are called from Rust via `reqwest`, which isn't subject to the webview's `connect-src`
CSP — confirmed nothing in the frontend calls Google's translate domains directly today.

## Verification

1. `cd src-tauri && cargo test` — updated `translate.rs` tests + new `dictionary.rs` parsing tests.
2. `cd src-tauri && cargo build` — confirms command signature/registration changes compile.
3. `npx tsc --noEmit` — confirms `SpeechHandle` typing and service changes are consistent across all call sites.
4. `npm run tauri dev`, then manually:
   - Add a Google Cloud Translation API key in Settings.
   - Translate a short word (e.g. "hello", en→vi): confirm translation succeeds (no 429), the dictionary panel
     shows definitions/examples/synonyms/pronunciation, and both speaker buttons produce audio via an OS voice.
   - Translate a full sentence: confirm it succeeds and the (now-absent-data) dictionary panel stays hidden.
   - Clear the API key and retranslate: confirm a clear "add an API key in Settings" error instead of a raw HTTP
     error.
