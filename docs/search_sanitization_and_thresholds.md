# Search Sanitization and Trigram Thresholds

KotaDB sanitizes every free-form query before it reaches the trigram indexes, and both the text and binary search engines enforce ratio-based match thresholds to keep false positives out of the result set.

## Step-by-Step: Standard Query Sanitization
1. Instantiate a `ValidationContext` so any failure or warning is tagged with the original text (`src/query_sanitization.rs:104`).
2. Reject oversized, null-containing, or control-heavy queries before further work (`src/query_sanitization.rs:111`, `src/query_sanitization.rs:119`).
3. Normalize whitespace and collapse control characters into spaces to produce a canonical working copy (`src/query_sanitization.rs:124`).
4. Strip SQL, command, path-traversal, and LDAP injection signatures using the compiled regex set while emitting warnings when patterns are removed (`src/query_sanitization.rs:133`, `src/query_sanitization.rs:141`, `src/query_sanitization.rs:148`, `src/query_sanitization.rs:155`).
5. Replace reserved characters like angle brackets, quotes, and null-ish bytes with spaces so the downstream parsers never see them (`src/query_sanitization.rs:166`).
6. If the build enables the `strict-sanitization` feature, delete standalone SQL keywords and aggressively strip characters that commonly open LDAP payloads (`src/query_sanitization.rs:184`).
7. Sweep for `..` and their percent-encoded counterparts to defeat late path-traversal attempts and then re-normalize whitespace (`src/query_sanitization.rs:203`).
8. Derive the final term list, keeping wildcard patterns that have real characters and trimming the count to `MAX_QUERY_TERMS` so the search planner stays bounded (`src/query_sanitization.rs:218`).
9. Warn when more than half the incoming terms were discarded and rebuild the query text from the surviving tokens, preserving a literal `*` wildcard if that was the original request (`src/query_sanitization.rs:243`, `src/query_sanitization.rs:253`).
10. Fail the call if sanitization produced an empty string (unless the user explicitly asked for `*`), ensuring upstream services never run empty searches unintentionally (`src/query_sanitization.rs:261`).

> **Note** `SanitizedQuery::was_modified` and `SanitizedQuery::warnings` surface everything the sanitizer touched so callers can log or metric anomalies without re-parsing the payload (`src/query_sanitization.rs:78`).

## Step-by-Step: Path-Aware Query Sanitization
1. Use the same `ValidationContext` and length/null guards as the standard sanitizer to keep diagnostics uniform (`src/query_sanitization.rs:276`).
2. Normalize whitespace but preserve forward slashes so paths survive the cleanup (`src/query_sanitization.rs:298`).
3. Remove SQL patterns everywhere but only trim shell metas when the query does not already include `/`, which prevents legitimate `find ./src`-style queries from being gutted (`src/query_sanitization.rs:307`, `src/query_sanitization.rs:315`).
4. Apply the LDAP regex and reserved-character sweep, but explicitly keep `/`, `*`, `()`, `[]`, `=`, `,`, `-`, and `_` because they regularly show up in file names and globbing syntax (`src/query_sanitization.rs:329`, `src/query_sanitization.rs:337`).
5. Produce terms by splitting on whitespace, keeping anything under `MAX_TERM_LENGTH`, and return a wildcard if every part was stripped so callers can gracefully handle empty results (`src/query_sanitization.rs:367`, `src/query_sanitization.rs:378`).

## Step-by-Step: Strict Mode Hardening (`strict-sanitization`)
1. Compile with `--features strict-sanitization` to activate the optional branch guarded by `cfg!(feature = "strict-sanitization")` (`src/query_sanitization.rs:184`).
2. Remove standalone SQL keywords regardless of context, preventing sequences like `select config` from slipping through even when no full pattern is present (`src/query_sanitization.rs:186`).
3. Strip `()`, `\`, `,`, and `=` characters that often anchor LDAP or shell payloads, shrinking the attack surface at the cost of more aggressive pruning (`src/query_sanitization.rs:192`).
4. Re-run the path-traversal sweep so the stricter output still benefits from the same canonicalization pass (`src/query_sanitization.rs:203`).

## Step-by-Step: Query Builder and Validation Flow
1. `QueryBuilder::with_text` inspects the raw text and routes it to the appropriate sanitizer, picking the path-aware variant whenever the payload looks like a filesystem query (`src/builders.rs:156`).
2. If the sanitizer reports an empty result while the input was not a wildcard, `QueryBuilder` aborts the build so callers cannot accidentally launch empty searches (`src/builders.rs:172`).
3. Any warnings are logged at debug level, capturing the original and sanitized text plus the warning list for observability (`src/builders.rs:177`).
4. The resulting string feeds into `Query::new`, which calls the sanitizer again for backwards-compatible pathways, then wraps every surviving term in `ValidatedSearchQuery` to apply length requirements (`src/contracts/mod.rs:205`, `src/contracts/mod.rs:215`, `src/types.rs:317`).
5. `ValidatedSearchQuery::new` enforces a configurable minimum length and caps the final string at 1024 characters, guaranteeing downstream index code receives clean, bounded input (`src/types.rs:317`).

## Step-by-Step: Classic Trigram Thresholding (`TrigramIndex`)
1. `TrigramIndex::search` extracts trigrams from each sanitized search term and accumulates document hit counts in memory (`src/trigram_index.rs:720`).
2. When no hits are found the method exits early, short-circuiting immediately for nonsense queries (`src/trigram_index.rs:761`).
3. For 1–3 trigrams the index demands a 100 % match; for 4–6 it uses an 80 % ceiling while still requiring at least `N-1`; beyond that it enforces roughly 60 % with a floor of three hits (`src/trigram_index.rs:773`).
4. Candidates falling below the computed threshold are culled before ranking, preventing random overlap in long documents from polluting results (`src/trigram_index.rs:788`).
5. Enabling the `aggressive-trigram-thresholds` feature unlocks a fallback ladder (2/3 → 1/3 → minimum hits depending on query length) to rescue legitimate queries that were over-filtered (`src/trigram_index.rs:795`).

> **Warning** The fallback ladder only runs when the strict threshold removed every candidate; keep the feature disabled in hostile environments to avoid reintroducing noisy matches (`src/trigram_index.rs:795`).

## Step-by-Step: Binary Trigram Thresholding (`BinaryTrigramIndex`)
1. `BinaryTrigramIndex::search` deduplicates trigrams up front so repeated fragments do not inflate hit counts before scoring (`src/binary_trigram_index.rs:482`).
2. It applies the same tiered threshold policy as the classic index, using integer `div_ceil` math to keep ratios precise in the binary format (`src/binary_trigram_index.rs:490`).
3. A single long or digit-heavy token triggers a stricter ≈90 % requirement to avoid broad matches on identifiers such as commit hashes (`src/binary_trigram_index.rs:505`).
4. The method looks in the hot in-memory cache first, then falls back to the memory-mapped store, accumulating scores per document (`src/binary_trigram_index.rs:523`, `src/binary_trigram_index.rs:535`).
5. After filtering by the computed threshold, results are sorted by match count and truncated to the query limit to keep pagination predictable (`src/binary_trigram_index.rs:555`).
6. `Database::new` selects the binary implementation when `use_binary_index` is true, otherwise defaulting to the classic index, so deployments can switch strategies without touching call sites (`src/database.rs:32`).

## Key Symbols and Locations
| Symbol | Purpose | Location |
| --- | --- | --- |
| `SanitizedQuery` | Carries sanitized text, term list, and mutation flags for logging and planning | `src/query_sanitization.rs:78` |
| `QueryBuilder::with_text` | Chooses the correct sanitizer and applies safety checks before building the query | `src/builders.rs:156` |
| `TrigramIndex::search` | Enforces ratio thresholds for the text-backed index | `src/trigram_index.rs:720` |
| `BinaryTrigramIndex::search` | Mirrors the threshold logic for the memory-mapped index | `src/binary_trigram_index.rs:475` |

## Next Steps
- Run `just test` to verify sanitizer and threshold regression suites still pass before shipping changes.
- Toggle `--features strict-sanitization` or `--features aggressive-trigram-thresholds` in a staging build to evaluate the impact on your workloads.
- Review the structured warnings emitted by `SanitizedQuery` in application logs to monitor real-world sanitization events.
