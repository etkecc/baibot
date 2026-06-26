# (2026-06-26) Version 1.24.0

- (**Feature**) Add an opt-in 💭 **thinking notice** for text generation. When enabled, a slow response (for example, from a reasoning model that runs for minutes) posts a "thinking…" placeholder after a short delay, refreshes it periodically with varying flavor text, and then edits that same message into the final answer, so a long wait no longer looks like a stuck bot. The notice is **disabled by default** and configurable per-room or globally via `text-generation set-thinking-notice-enabled true`. Fast responses (under the delay threshold) never show a placeholder. See the [text-generation configuration docs](./docs/configuration/text-generation.md#-thinking-notice).

- (**Bugfix**) The [Venice](https://venice.ai) unsupported-field auto-recovery (added in 1.23.1) now actually remembers rejections across messages. The cache lived on the provider's controller, which is rebuilt on every message for room-local and global agents, so each turn started with an empty cache, re-sent the unsupported field, and logged the same `400 Bad Request` warning again. The cache is now process-global (keyed per Venice deployment), so a field a model rejects is dropped proactively on every later request instead of being re-discovered each turn.


# (2026-06-24) Version 1.23.1

- (**Bugfix**) The [Venice](https://venice.ai) provider now auto-recovers when a model rejects an optional knob it does not support. Venice's request body is strict (`additionalProperties: false`), so a model that lacks `prompt_cache_retention`, `reasoning_effort`, or `prompt_cache_key` rejected the whole request with a `400 Bad Request` — breaking agent creation and every reply. baibot now drops the unsupported field and retries, remembering the rejection per model so later requests skip it without a wasted round-trip. Only these meaning-preserving fields are dropped; sampling knobs that change the output (`temperature`, `top_p`, the penalties) are never silently removed and still surface as an error.

- (**Improvement**) When Venice rejects a request with a `400 Bad Request`, baibot now surfaces Venice's actual error message (e.g. `Extra inputs are not permitted, field: 'prompt_cache_retention'`) instead of a generic "configuration does not result in a working agent". This makes agent-creation failures self-explanatory. Other error statuses keep their bodies redacted, since those can carry account or rate-limit details.


# (2026-06-23) Version 1.23.0

- (**Feature**) The [Venice](https://venice.ai) provider now accepts file inputs (PDF, DOCX, and other documents, up to 25MB), the same way it already handled images. This makes Venice the second provider after OpenAI to accept files; the others (Anthropic and the OpenAI-compatible providers) skip them. See the [text-generation feature docs](./docs/features.md#-text-generation).

- (**Feature**) Add prompt caching to the Venice provider, on by default (`prompt_cache_retention: 24h`). baibot derives the cache key from the system prompt and the conversation start time (both fixed for the life of a conversation), so a long, stable system prompt stays cached across the day instead of being reprocessed and re-billed on every turn. See [Text Generation / Prompt Override](./docs/configuration/text-generation.md#️-prompt-override).

- (**Feature**) Wire up the rest of Venice's sampling and reasoning controls: top-level `top_p`, `frequency_penalty`, `presence_penalty`, `repetition_penalty`, and `reasoning_effort`; `verbosity` in the `venice_parameters` bag; and a `show_reasoning` toggle that appends the model's reasoning to the reply as a collapsible, folded-by-default `💭 Reasoning` block (off by default). See the [Venice configuration reference](./docs/providers.md#venice).

- (**Feature**) Render Venice web-search citations as readable `[n]` references with a `Sources:` list of links, instead of leaving Venice's raw `^n^` superscripts in the reply.

- (**Security**) Escape citation titles and validate citation URLs before rendering them, and drop user-supplied filenames from error messages, so a hostile web page or a crafted filename cannot inject a spoofed link into the bot's reply.

- (**Bugfix**) The [OpenAI-compatible](./docs/providers.md#openai-compatible) provider now trusts the system CA store (honoring `SSL_CERT_FILE`), so endpoints served behind a private/internal CA (FreeIPA, organization PKI) no longer fail the TLS handshake with `invalid peer certificate: UnknownIssuer`. Fixed upstream in `etke_openai_api_rust` 0.1.10. Thanks to [@shaba](https://github.com/shaba) for the report in [#188](https://github.com/etkecc/baibot/pull/188).


# (2026-06-21) Version 1.22.0

- (**Feature**) Add a native [Venice](https://venice.ai) provider with [🖌️ image-generation](./docs/features.md#️-image-creation) (incl. editing), [💬 text-generation](./docs/features.md#-text-generation) (incl. vision), [🗣️ text-to-speech](./docs/features.md#️-text-to-speech), [🦻 speech-to-text](./docs/features.md#-speech-to-text), and Venice's native web search via the full `venice_parameters` knob set. Unlike the [OpenAI-compatible](./docs/providers.md#openai-compatible) path (which drops images and can't reach Venice's audio or native image endpoints), it talks to Venice's API directly, using the knob-rich native `/image/generate` and `/image/edit` endpoints. See the [Venice provider docs](./docs/providers.md#venice).


# (2026-06-05) Version 1.21.1

- (**Security**) Update the [anthropic](https://github.com/etkecc/anthropic-rs) dependency to use [reqwest](https://crates.io/crates/reqwest) 0.12 / [rustls](https://crates.io/crates/rustls) 0.23, replacing the vulnerable `rustls-webpki` 0.101 line with 0.103.13. This resolves [`GHSA-82j2-j2ch-gfr8`](https://github.com/advisories/GHSA-82j2-j2ch-gfr8) (high — denial of service via panic on a malformed CRL), [`GHSA-xgp8-3hg3-c2mh`](https://github.com/advisories/GHSA-xgp8-3hg3-c2mh) and [`GHSA-965h-392x-2mh5`](https://github.com/advisories/GHSA-965h-392x-2mh5) (name-constraint validation issues).


# (2026-06-05) Version 1.21.0

- (**Improvement**) Default to OpenAI's `gpt-image-2` model for image generation (in newly-created OpenAI agents and the sample provider configs).

- (**Internal Improvement**) Update [async-openai](https://crates.io/crates/async-openai) from 0.40 to 0.41, which [resynchronizes with the upstream OpenAI API spec](https://github.com/64bit/async-openai/issues/557) after it had drifted out of sync — a mismatch that was already causing some breakage (hopefully now resolved). Adapts to the newly-added `gpt-image-2` image model and an `ImageSize` type change.

- (**Internal Improvement**) Dependency updates.


# (2026-06-02) Version 1.20.0

- (**Internal Improvement**) Update [matrix-sdk](https://crates.io/crates/matrix-sdk) from 0.17 to 0.18 and [mxlink](https://crates.io/crates/mxlink) to 1.15.0.

- (**Internal Improvement**) Update [tiktoken-rs](https://crates.io/crates/tiktoken-rs) to 0.12, backporting OpenAI [tiktoken](https://github.com/openai/tiktoken) 0.13.0 for better alignment with upstream tokenization behavior.

- (**Internal Improvement**) Bump the pinned Rust toolchain from 1.95.0 to 1.96.0 (in `rust-toolchain.toml` and the Docker build images).

- (**Internal Improvement**) Dependency updates.


# (2026-05-27) Version 1.19.3

- (**Internal Improvement**) Update [async-openai](https://crates.io/crates/async-openai) to 0.40.2, pulling in several upstream fixes (streaming HTTP error surfacing, default `ResponseTextParam.format` deserialization, etc.).

- (**Internal Improvement**) Dependency updates.


# (2026-05-21) Version 1.19.2

- (**Internal Improvement**) Update [async-openai](https://crates.io/crates/async-openai) to 0.40.0.

- (**Internal Improvement**) Dependency updates.


# (2026-05-09) Version 1.19.1

- (**Internal Improvement**) Update [async-openai](https://crates.io/crates/async-openai) to 0.38.0.

- (**Internal Improvement**) Dependency updates.


# (2026-05-09) Version 1.19.0

- (**Internal Improvement**) Update [matrix-sdk](https://crates.io/crates/matrix-sdk) from 0.16 to 0.17 and [mxlink](https://crates.io/crates/mxlink) to 1.14.0. matrix-sdk 0.17 dropped its `native-tls` feature and now uses [rustls](https://github.com/rustls/rustls) exclusively as its TLS backend.

- (**Internal Improvement**) Bump the pinned Rust toolchain from 1.93.0 to 1.95.0 (in `rust-toolchain.toml` and the Docker build images).

- (**Internal Improvement**) Dependency updates.


# (2026-04-11) Version 1.18.0

- (**Bugfix**) Fix the bot not sending a welcome message when joining a room on homeservers (like [Continuwuity](https://continuwuity.org/)) that place the join membership event in the sync response's `state` block rather than the `timeline` block, via [mxlink](https://crates.io/crates/mxlink) 1.13.1

- (**Improvement**) Update [tiktoken-rs](https://crates.io/crates/tiktoken-rs) to 0.11, adding tokenization support for newer GPT models (gpt-5.x, codex, etc.) and fixing context sizes for o1-mini/chatgpt-4o/gpt-4.5

- (**Internal Improvement**) Dependency updates


# (2026-03-25) Version 1.17.0

- (**Feature**) Add `text-generation sender-context-mode` for attaching sender metadata to conversation messages. See the [💬 Text Generation](./docs/configuration/text-generation.md#-sender-context-mode) documentation for details. Thanks to [kschwank](https://github.com/kschwank) for the contribution in [#104](https://github.com/etkecc/baibot/pull/104)!


# (2026-03-24) Version 1.16.1

- (**Bugfix**) Fix compatibility with [async-openai](https://crates.io/crates/async-openai) 0.34.0 by populating the new `phase` field required for OpenAI Responses API message inputs. baibot does not currently distinguish between assistant `commentary` and `final_answer` turns, so using `None` preserves the previous behavior while remaining compatible with the updated crate.

- (**Internal Improvement**) Dependency updates.


# (2026-03-20) Version 1.16.0

- (**Feature**) Add support for file attachments (`m.file` Matrix messages) in conversations. Files like PDFs, text documents, spreadsheets, code files, etc. are now downloaded and forwarded to the LLM alongside the conversation context, similar to how images (`m.image`) are already handled. See the [💬 Text Generation](./docs/features.md#-text-generation) documentation for details and known limitations.

- (**Improvement**) Use the [mime_guess](https://crates.io/crates/mime_guess) crate for MIME type detection from file extensions, replacing a hand-maintained mapping. This covers hundreds of file extensions out of the box.



# (2026-03-07) Version 1.15.0

- (**Feature**) Add support for authentication via access tokens (for [Matrix Authentication Service](https://github.com/element-hq/matrix-authentication-service)/OIDC-enabled homeservers) as an alternative to password authentication. See [🔐 Authentication](./docs/configuration/authentication.md) for setup details. Thanks to [Taylor Southwick](https://github.com/twsouthwick) for the contribution in [#83](https://github.com/etkecc/baibot/pull/83)!

- (**Internal Improvement**) Pin the Rust toolchain to `1.93.0` in both CI and local development to avoid `matrix-sdk` build failures on newer stable toolchains.

- (**Internal Improvement**) Documentation updates.

- (**Internal Improvement**) Dependency updates.


# (2026-02-18) Version 1.14.3

- (**Internal Improvement**) Add [Renovate](https://docs.renovatebot.com/) configuration for automated dependency updates

- (**Internal Improvement**) Dependency updates


# (2026-02-18) Version 1.14.2

- (**Internal Improvement**) Dependency updates

- (**Internal Improvement**) Reorganize the development environment to support [Continuwuity](https://continuwuity.org/) as a homeserver choice (in addition to [Synapse](https://github.com/element-hq/synapse)). Continuwuity is now the default for its lighter footprint (no external database required). See [development docs](./docs/development.md) for details.


# (2026-02-10) Version 1.14.1

- (**Security**) Dependency updates to fix security vulnerabilities ([time](https://crates.io/crates/time) stack exhaustion DoS, [bytes](https://crates.io/crates/bytes) integer overflow), via [mxlink](https://crates.io/crates/mxlink) 1.12.0

- (**Internal Improvement**) Switch from deprecated [serde_yaml](https://crates.io/crates/serde_yaml) to its maintained fork [serde_yaml_ng](https://crates.io/crates/serde_yaml_ng)

- (**Internal Improvement**) Add [prek](https://github.com/nicholasgasior/prek) pre-commit hooks via [mise](https://mise.jdx.dev/) for automated code quality checks (formatting, clippy, tests)

- (**Internal Improvement**) Fix clippy warnings and formatting issues


# (2026-02-04) Version 1.14.0

- (**Feature**) The `openai` provider now uses OpenAI's [Responses API](https://platform.openai.com/docs/api-reference/responses) (instead of the older Chat Completions API), adding support for [🛠️ built-in tools](./docs/features.md#️-built-in-tools-openai-only) (`web_search` and `code_interpreter`). These tools are **disabled by default** and can be enabled via the `text_generation.tools` configuration (see the [sample configuration](https://github.com/etkecc/baibot/blob/c70387b0c38d8d0f30bba2179a2a21a3710dbeaf/docs/sample-provider-configs/openai.yml#L12-L15)). To enable tools on an existing agent, you need to [update the agent](./docs/agents.md#updating-agents) to re-create it with the `text_generation.tools` section added and enable the tools you need. Thanks to [Layla Manley](https://github.com/yeslayla) for the contribution in [#62](https://github.com/etkecc/baibot/pull/62)!

- (**Bugfix**) Fix sticker generation for newer GPT image models (`gpt-image-1`, `gpt-image-1-mini`, `gpt-image-1.5`) which don't support the previously hardcoded `256x256` size (minimum is `1024x1024`)

- (**Internal Improvement**) Dependency updates


# (2026-01-23) Version 1.13.0

- (**Improvement**) Extend auto-switching to support cheaper models (`gpt-image-1-mini`) for `gpt-image-1` and `gpt-image-1.5` when generating stickers ([e0b4a40](https://github.com/etkecc/baibot/commit/e0b4a40))

- (**Internal Improvement**) Upgrade Rust compiler (1.92.0 -> 1.93.0) ([691aeeb](https://github.com/etkecc/baibot/commit/691aeeb))

- (**Internal Improvement**) Dependency updates


# (2025-12-21) Version 1.12.0

- (**Improvement**) Upgrade [async-openai](https://crates.io/crates/async-openai) (0.31.1 -> 0.32.2) and add support for OpenAI's `gpt-image-1.5` model ([08c689a](https://github.com/etkecc/baibot/commit/08c689a), [f7bf3d7](https://github.com/etkecc/baibot/commit/f7bf3d7))

- (**Internal Improvement**) Dependency updates


# (2025-12-15) Version 1.11.0

- (**Feature**) Add support for custom avatars via file path and for keeping the already-set avatar (for those who wish to manage it by themselves via other means). See the [sample config](./etc/app/config.yml.dist) for details. ([062fbbb](https://github.com/etkecc/baibot/commit/062fbbb8ef9ad600db483a431c5c782402191023))

- (**Internal Improvement**) Dependency updates ([99bde53](https://github.com/etkecc/baibot/commit/99bde53ef648a5a9086a96778fde4a9dbc1ede58))

- (**Internal Improvement**) Documentation updates ([b3fd8e5](https://github.com/etkecc/baibot/commit/b3fd8e548f83fe46398ced4760d7e2bb7588c24d))

- (**Internal Improvement**) Upgrade Rust compiler (1.91.1 -> 1.92.0) ([22906aa](https://github.com/etkecc/baibot/commit/22906aa2d3cae51815fad2560a545eaa69c247b6))


# (2025-12-06) Version 1.10.0

- (**Internal Improvement**) Dependency updates. This version is based on [mxlink](https://crates.io/crates/mxlink)@1.11.0 (which is based on the newly released [matrix-sdk](https://crates.io/crates/matrix-sdk)@[0.16.0](https://github.com/matrix-org/matrix-rust-sdk/releases/tag/matrix-sdk-0.16.0).

# (2025-11-30) Version 1.9.0

- (**Internal Improvement**) Upgrade [async-openai](https://crates.io/crates/async-openai) from our own etkecc fork (0.28.1-patched) to the official upstream version 0.31.1. This upgrade required some code adaptations to the new module structure, etc. While tested, regressions are possible.

# (2025-11-28) Version 1.8.3

- (**Improvement**) Add support for the `BAIBOT_PERSISTENCE_SESSION_ENCRYPTION_KEY` environment variable for configuring `persistence.session_encryption_key`

- (**Improvement**) Add support for the `BAIBOT_USER_ENCRYPTION_RECOVERY_RESET_ALLOWED` environment variable for configuring `user.encryption.recovery_reset_allowed`

- (**Internal Improvement**) Dependency updates.

# (2025-11-20) Version 1.8.2

- (**Internal Improvement**) Dependency and compiler updates (Rust 1.89.0 -> 1.91.1).

# (2025-09-12) Version 1.8.1

- (**Internal Improvement**) Dependency updates.

# (2025-09-08) Version 1.8.0

- (**Internal Improvement**) Upgrade [mxlink](https://crates.io/crates/mxlink) (1.9.0 -> 1.10.0) and [matrix-sdk](https://crates.io/crates/matrix-sdk) (0.13.0 -> 0.14.0)

- (**Internal Improvement**) Upgrade [Rust](https://www.rust-lang.org/) (1.88.0 -> 1.89.0)

- (**Internal Improvement**) Upgrade Debian base for container images (12/bookworm -> 13/trixie)

# (2025-07-11) Version 1.7.6

- (**Internal Improvement**) Dependency updates. This version is based on [mxlink](https://crates.io/crates/mxlink)@1.9.0 (which is based on the newly released [matrix-sdk](https://crates.io/crates/matrix-sdk)@[0.13.0](https://github.com/matrix-org/matrix-rust-sdk/releases/tag/matrix-sdk-0.13.0), which contains fixes for some security vulnerabilities)

# (2025-06-10) Version 1.7.5

- (**Internal Improvement**) Dependency and compiler updates (Rust 1.86 -> 1.86).

# (2025-06-10) Version 1.7.4

- (**Internal Improvement**) Dependency updates.

# (2025-06-10) Version 1.7.3

- (**Internal Improvement**) Dependency updates. This version is based on [mxlink](https://crates.io/crates/mxlink)@1.8.0 (which is based on the newly released [matrix-sdk](https://crates.io/crates/matrix-sdk)@[0.12.0](https://github.com/matrix-org/matrix-rust-sdk/releases/tag/matrix-sdk-0.12.0), which contains fixes for important security vulnerabilities)

# (2025-05-11) Version 1.7.2

- (**Bugfix**) Allow `image_generation.size` configuration value for OpenAI to be `null` to allow the model to choose the size automatically and default to that

# (2025-05-11) Version 1.7.1

- (**Bugfix**) Fix lack of documentation for the new [image-editing](./docs/features.md#-image-editing) feature in the `!bai usage` command's output

# (2025-05-10) Version 1.7.0

- (**Feature**) Add vision support to the OpenAI and Anthropic providers. You can now mix text and images in your conversations - fixes [issue #5](https://github.com/etkecc/baibot/issues/5)

- (**Feature**) Add [image-editing](./docs/features.md#-image-editing) support to the OpenAI provider

- (**Improvement**) Add compatibility with OpenAI's `gpt-image-1` model - fixes [issue #40](https://github.com/etkecc/baibot/issues/40)

- (**Change**) Rework [image-creation](./docs/features.md#-image-creation) to avoid command conflicts with [image-editing](./docs/features.md#-image-editing). The image-creation command syntax is now `!bai image create <prompt>` (previously: `!bai image <prompt>`).

- (**Internal Improvement**) Dependency and compiler updates

> [!WARNING]
> Unlike other releases, this release is not published to [crates.io](https://crates.io), because it relies on multiple library forks (`async-openai` and `anthropic-rs`) sourced from Github.


# (2025-04-12) Version 1.6.0

- (**Internal Improvement**) Dependency updates. This version is based on [mxlink](https://crates.io/crates/mxlink)@1.7.0 (which is based on the newly released [matrix-sdk](https://crates.io/crates/matrix-sdk)@[0.11.0](https://github.com/matrix-org/matrix-rust-sdk/releases/tag/matrix-sdk-0.11.0))


# (2025-03-31) Version 1.5.1

- (**Internal Improvement**) Dependency updates

# (2025-02-27) Version 1.5.0

- (**Feature**) Add support for sending Speech-to-Text replies for [Transcribe-only mode](./docs/features.md#transcribe-only-mode) as regular text messages instead of notices and doing it so by default ([a1bd292752](https://github.com/etkecc/baibot/commit/a1bd292752bdd37a196788c73d00b5619e843a78)) - improvement for [issue #14](https://github.com/etkecc/baibot/issues/14). See [🦻 Speech-to-Text / 🪄 Message Type for non-threaded only-transcribed messages](./docs/configuration/speech-to-text.md#-message-type-for-non-threaded-only-transcribed-messages) for details.

- (**Feature**) Add config setting controlling if a self-introduction message is posted after joining a room ([c051da2f4a](https://github.com/etkecc/baibot/commit/c051da2f4a161de0974ebb917f7a52d01f5a001f)) - fixes [issue #32](https://github.com/etkecc/baibot/issues/32). You may wish to add a `room.post_join_self_introduction_enabled` property to your configuration. See the [sample config](./etc/app/config.yml.dist) for details. If unspecified, it defaults to `true` anyway which preserves the old behavior.

- (**Feature**) Add support for configuring `max_completion_tokens` for OpenAI ([47d8edea70](https://github.com/etkecc/baibot/commit/47d8edea705a44aa25a9bfaec4888c0f9ea8700e))

- (**Improvement**) Dependency updates. This version is based on [mxlink](https://crates.io/crates/mxlink)@1.6.1 (which is based on the newly released [matrix-sdk](https://crates.io/crates/matrix-sdk)@[0.10.0](https://github.com/matrix-org/matrix-rust-sdk/releases/tag/matrix-sdk-0.10.0))

- (**Improvement**) Populate image/audio attachment `body` with a filename, not with text to avoid incorrect rendering in Element Web, etc. ([ec1879d212](https://github.com/etkecc/baibot/commit/ec1879d212fa8d6e5f8590486e94c72abfcb75a5))

- (**Improvement**) Replace Anthropic library ([anthropic-rs](https://crates.io/crates/anthropic-rs) -> [anthropic](https://crates.io/crates/anthropic)) and switch default recommended model (`claude-3-5-sonnet-20240620` -> `claude-3-7-sonnet-20250219`) ([692d61b239](https://github.com/etkecc/baibot/commit/692d61b2398f073b81d32d4cbe8145ab3929e48c)) - fixes [issue #22](https://github.com/etkecc/baibot/issues/22)

- (**Internal Improvement**) Switch to native building of `arm64` container images to decrease total build times from ~40 minutes to ~8 minutes ([6719538530b](https://github.com/etkecc/baibot/commit/6719538530bf76b3ff2d24077b2a7fa868276b79))

- (**Internal Improvement**) Various other internal changes, including upgrading [Rust from 1.82 to 1.85 and switching to Rust edition 2024](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)


# (2024-12-12) Version 1.4.1

- (**Bugfix**) Fix detection for whether the bot is the last member in a room, to avoid incorrectly leaving multi-user rooms that have had at least one person `leave` ([3c47d40781](https://github.com/etkecc/baibot/commit/3c47d407819aa9c0121117a411858238724f06da))


# (2024-11-19) Version 1.4.0

- (**Improvement**) Dependency updates. This version is based on [mxlink](https://crates.io/crates/mxlink)@1.4.0 (which is based on the newly released [matrix-sdk](https://crates.io/crates/matrix-sdk)@[0.8.0](https://github.com/matrix-org/matrix-rust-sdk/releases/tag/matrix-sdk-0.8.0)). Once you run this version at least once and your matrix-sdk datastore gets upgraded to the new schema, **you will not be able to downgrade to older baibot versions** (based on the older matrix-sdk), unless you start with an empty datastore.

- (**Bugfix**) Add missing typing notices sending functionality while generating images ([9d166e35ba](https://github.com/etkecc/baibot/commit/9d166e35ba6fc0daaf69318870e92436f3302056))

- (**Feature**) Support for [Matrix authenticated media](https://matrix.org/docs/spec-guides/authed-media-servers/), thanks to upgrading [mxlink](https://crates.io/crates/mxlink) / [matrix-sdk](https://crates.io/crates/matrix-sdk) - fixes [issue #12](https://github.com/etkecc/baibot/issues/12)


# (2024-11-12) Version 1.3.2

Dependency updates.


# (2024-10-03) Version 1.3.1

- (**Improvement**) Improves fallback user mentions support for old clients (like Element iOS) which use the bot's display name (not its full Matrix User ID). ([d9a045a5e4](https://github.com/etkecc/baibot/commit/d9a045a5e41d2b99694f92ec9e90f47529546d89))


# (2024-10-03) Version 1.3.0

**TLDR**: you can now use OpenAI's [o1](https://platform.openai.com/docs/models/o1) models, benefit from [prompt caching](https://platform.openai.com/docs/guides/prompt-caching) and mention the bot again from old clients lacking proper [user mentions support](https://spec.matrix.org/latest/client-server-api/#user-and-room-mentions) (like Element iOS).

- (**Feature**) Introduces a new `baibot_conversation_start_time_utc` [prompt variable](./docs/configuration/text-generation.md#️-prompt-override) which is not a moving target (like the `baibot_now_utc` variable) and allows [prompt caching](https://platform.openai.com/docs/guides/prompt-caching) to work. All default/sample configs have been adjusted to make use of this new variable, but users need to adjust your existing dynamically-created agents to start using it. ([85e66406dc](https://github.com/etkecc/baibot/commit/85e66406dc6f430741c7819f420e2df4ae6e8d3b))

- (**Improvement**) Allows for the `max_response_tokens` configuration value for the [OpenAI provider](./docs/providers.md#openai) to be set to `null` to allow [o1](https://platform.openai.com/docs/models/o1) models (which do not support `max_response_tokens`) to be used. See the new o1 sample config [here](./docs/sample-provider-configs/openai-o1.yml). ([db9422740c](https://github.com/etkecc/baibot/commit/db9422740ceca32956d9628b6326b8be206344e2))

- (**Improvement**) Switches the sample configs for the [OpenAI provider](./docs/providers.md#openai) to point to the `gpt-4o` model, which since 2024-10-02 is the same as the `gpt-4o-2024-08-06` model. We previously explicitly pointed the bot to the `gpt-4o-2024-08-06` model, because it was much better (longer context window). Now that `gpt-4o` points to the same powerful model, we don't need to pin its version anymore. Existing users may wish to adjust their configuration to match. ([90fbad5b64](https://github.com/etkecc/baibot/commit/90fbad5b643cd06c23179f055a309ec6a7cba161))

- (**Bugfix**) Restores fallback user mentions support (via regular text, not via the [user mentions spec](https://spec.matrix.org/latest/client-server-api/#user-and-room-mentions)) to allow certain old clients (like Element iOS) to be able to mention the bot again. Support for this was intentionally removed recently (in [v1.2.0](#2024-10-01-version-120)), but it turned out to be too early to do this. ([b40226826f](https://github.com/etkecc/baibot/commit/b40226826fe914d0d5d265230ebc5bac8058b6f7))


# (2024-10-01) Version 1.2.0

- (**Feature**) Adds support for [on-demand involvement](./docs/features.md#on-demand-involvement) of the bot (via mention) in arbitrary threads and reply chains ([9908512968](https://github.com/etkecc/baibot/commit/990851296828168c2106eb3f4668833e9e5a7463)) - fixes [issue #15](https://github.com/etkecc/baibot/issues/15)

- (**Improvement**) Simplifies [Transcribe-only mode](./docs/features.md#transcribe-only-mode) reply format (removing `> 🦻` prefixing) to allow easier forwarding, etc. ([e6aa956423](https://github.com/etkecc/baibot/commit/e6aa95642376ee7d87932d0e66dcfedf261b188b)) - fixes [issue #14](https://github.com/etkecc/baibot/issues/14)

- (**Bugfix**) Fixes speech-to-text replies rendering incorrectly in certain clients, due to them confusing our old reply format with [fallback for rich replies](https://spec.matrix.org/v1.11/client-server-api/#fallbacks-for-rich-replies) ([e6aa956423](https://github.com/etkecc/baibot/commit/e6aa95642376ee7d87932d0e66dcfedf261b188b)) - fixes [issue #17](https://github.com/etkecc/baibot/issues/17)


# (2024-09-22) Version 1.1.1

- (**Bugfix**) Fix thread messages being lost due to lack of pagination support ([d4ddd29660](https://github.com/etkecc/baibot/commit/d4ddd29660d9f51d248119dd6032e68ab29e7d35)) - fixes [issue #13](https://github.com/etkecc/baibot/issues/13)

- (**Bugfix**) Fix Anthropic conversations getting stuck when being impatient and sending multiple consecutive messages ([8b12bdf2b3](https://github.com/etkecc/baibot/commit/8b12bdf2b3196abea0e8db33d7c50fff48341cb9)) - fixes [issue #13](https://github.com/etkecc/baibot/issues/13)


# (2024-09-21) Version 1.1.0

- (**Feature**) Adds support for [prompt variables](./docs/configuration/text-generation.md#️-prompt-override) (date/time, bot name, model id) ([2a5a2d6a4d](https://github.com/etkecc/baibot/commit/2a5a2d6a4dbf5fd7cb504ac07d4187fdc32ae395)) - fixes [issue #10](https://github.com/etkecc/baibot/issues/10)

- (**Improvement**) [Dockerfile](./Dockerfile) changes to produce ~20MB smaller container images ([354063abb7](https://github.com/etkecc/baibot/commit/354063abb79035069bd3b26c53214874e9cdd95d))

- (**Improvement**) [Dockerfile](./Dockerfile) changes to optimize local (debug) runs in a container ([c8c5e0e540](https://github.com/etkecc/baibot/commit/c8c5e0e540ab981e849452eb3ddb0378105e1fc6))

- (**Improvement**) CI changes to try and work around multi-arch image issues like [this one](https://github.com/etkecc/baibot/issues/2) ([5de7559ed6](https://github.com/etkecc/baibot/commit/5de7559ed685a41c22dfc12283681f02f4c2ee00))


# (2024-09-19) Version 1.0.6

Improvements to:

- messages sent by the bot - better onboarding flow, especially when no agents have been created yet
- documentation pages


# (2024-09-14) Version 1.0.5

Further [improves](https://github.com/etkecc/baibot/commit/3b25b92a81a05ebaf1c6dbabf675fbfbe6c9f418) the typing notification logic, so that it tolerates edge cases better.


# (2024-09-14) Version 1.0.4

[Improves](https://github.com/etkecc/baibot/commit/dd1dd78312e3db7f92b37fb3b4750fbe35de7115) the typing notification logic.


# (2024-09-13) Version 1.0.3

Contains [fixes](https://github.com/etkecc/rust-mxlink/commit/f339fc85e69aa7f614394ad303d1614cd307319c) for [some](https://github.com/etkecc/baibot/issues/1) startup failures caused by partial initialization (errors during startup).


# (2024-09-12) Version 1.0.0

Initial release. 🎉
