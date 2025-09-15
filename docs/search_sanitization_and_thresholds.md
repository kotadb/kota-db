Search Query Sanitization and Trigram Thresholds

Overview

- KotaDB sanitizes search input to guard against injection while preserving developer-friendly queries.
- Two sanitization entry points exist:
  - `sanitize_search_query` — general text/code search (default-safe, preserves programming terms and common symbols).
  - `sanitize_path_aware_query` — path-oriented queries (preserves path characters like `/`, `=`, `(`, `)`, `[`, `]`, `,`, `-`, `_`).

Default Behavior (Recommended)

- Preserves standalone terms such as `create`, `select`, `insert`, etc. These are common in developer searches and are not removed unless used in SQL-like patterns.
- Blocks high-confidence injection patterns:
  - SQL: patterns like `union select`, `select ... from`, `insert into`, `update ... set`, `delete from`, `drop/create/alter table`.
  - Command injection indicators: `|`, `;`, `` ` ``, `$(`, etc.
  - Path traversal: `..`, encoded dot sequences like `%2e`, `%252e`.
  - Basic XSS patterns: `<script>`, `javascript:` URLs, common event handlers.
- Wildcards `*` are preserved; controls and reserved dangerous characters (`<`, `>`, `&`, quotes, null/CR/LF/TAB) are removed.

Strict Mode (Opt-in)

- Feature: enable with Cargo feature `strict-sanitization`.
- Additional behaviors:
  - Removes standalone SQL keywords regardless of context.
  - Strips additional characters in non‑path‑aware mode: `(`, `)`, `\\`, `,`, `=`.
- Intended for environments with elevated threat models; default builds keep this OFF to avoid breaking common code queries.

Binary vs Regular Trigram Index Thresholds

- Regular trigram index (default) applies ratio-based minimum-match filtering:
  - 1–3 trigrams: 100% required.
  - 4–6 trigrams: ~80% required (ceil), at least N−1.
  - 7+ trigrams: ~60% required (ceil), at least 3.
- Binary trigram index aligns with the same ratios and uses integer match counts for efficiency.
- Heuristic: single very long or digit-heavy token requires ~90% match to reduce false positives.

Choosing the Right Sanitizer

- Use `sanitize_search_query` for general text/code search when you want natural queries like `function(param)` or `config=value` to remain intact.
- Use `sanitize_path_aware_query` when queries include paths or path-like syntax. This preserves additional path characters while still removing dangerous patterns.

Notes

- The sanitizers normalize whitespace and enforce length/term count limits to prevent resource exhaustion.
- When upgrading, review integration tests under `tests/` and run `just ci-fast` locally.
