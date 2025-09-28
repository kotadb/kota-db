#!/usr/bin/env bash
set -euo pipefail

# Rewrites Markdown documentation under docs/ by delegating to Codex.
# Designed for both local prototyping and CI automation, with safeguards
# for selective execution and headless environments.

usage() {
  cat <<'USAGE'
Usage: scripts/codex_rewrite_docs.sh [OPTIONS] [DOC_PATH ...]

Environment variables:
  DRY_RUN=true|false          Preview changes without editing files (default: true)
  BYPASS_SANDBOX=true|false   Add --dangerously-bypass-approvals-and-sandbox
  CODEX_PROFILE=name          Codex profile to use (default: rewrite)
  CODEX_MODEL=name            Override model for this run
  CHANGED_ONLY=true|false     Rewrite only docs changed since DIFF_BASE (default: false)
  DIFF_BASE=<ref>             Base ref for change detection (default: origin/develop)
  DIFF_HEAD=<ref>             Head ref for change detection (default: HEAD)
  KEEP_PROMPTS=true|false     Persist generated prompts under .codex/run-prompts (default: false)
  SOURCE_REF=<ref>            Fallback git ref to read legacy docs from (default: HEAD)

Positional arguments limit the run to specific doc paths (must live under docs/).
USAGE
}

PROFILE="${CODEX_PROFILE:-rewrite}"
PROMPT_TEMPLATE="$(git rev-parse --show-toplevel)/.codex/prompts/docs_rewrite.md"
DRY_RUN="${DRY_RUN:-true}"
BYPASS_SANDBOX="${BYPASS_SANDBOX:-false}"
CODEX_MODEL="${CODEX_MODEL:-${MODEL:-}}"
CHANGED_ONLY="${CHANGED_ONLY:-false}"
DIFF_BASE="${DIFF_BASE:-origin/develop}"
DIFF_HEAD="${DIFF_HEAD:-HEAD}"
KEEP_PROMPTS="${KEEP_PROMPTS:-false}"
SOURCE_REF="${SOURCE_REF-HEAD}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if ! command -v codex >/dev/null 2>&1; then
  echo "codex CLI not found on PATH" >&2
  exit 1
fi

if [[ ! -f "$PROMPT_TEMPLATE" ]]; then
  echo "Prompt template not found: $PROMPT_TEMPLATE" >&2
  exit 1
fi

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

run_id="docs_rewrite_$(date +%Y%m%dT%H%M%S)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

if [[ "$KEEP_PROMPTS" == "true" ]]; then
  PROMPT_ARCHIVE=".codex/run-prompts/$run_id"
  mkdir -p "$PROMPT_ARCHIVE"
else
  PROMPT_ARCHIVE=""
fi

declare -a DOCS=()
USER_SPECIFIED=false

if (($# > 0)); then
  USER_SPECIFIED=true
  while (($# > 0)); do
    DOC_PATH="${1#./}"
    shift
    if [[ "$DOC_PATH" != docs/* ]]; then
      echo "Error: $DOC_PATH is outside docs/." >&2
      exit 1
    fi
    if [[ ! -f "$DOC_PATH" ]]; then
      if [[ -n "$SOURCE_REF" ]] && git cat-file -e "$SOURCE_REF:$DOC_PATH" 2>/dev/null; then
        :
      else
        echo "Warning: $DOC_PATH not found locally or in $SOURCE_REF; generating from scratch." >&2
      fi
    fi
    DOCS+=("$DOC_PATH")
  done
elif [[ "$CHANGED_ONLY" == "true" ]]; then
  while IFS= read -r doc; do
    [[ -n "$doc" ]] && DOCS+=("$doc")
  done < <(git diff --name-only "$DIFF_BASE" "$DIFF_HEAD" -- docs/ | awk '/\\.md$/')
else
  while IFS= read -r doc; do
    [[ -n "$doc" ]] && DOCS+=("$doc")
  done < <(find docs -type f -name '*.md' -print | sort)
fi

if [[ ${#DOCS[@]} -eq 0 && "$USER_SPECIFIED" == "false" && -n "$SOURCE_REF" ]]; then
  while IFS= read -r doc; do
    [[ -n "$doc" ]] && DOCS+=("$doc")
  done < <(git ls-tree --name-only -r "$SOURCE_REF" docs 2>/dev/null | sort)
fi

if [[ ${#DOCS[@]} -eq 0 ]]; then
  echo "No documentation files matched the selection criteria."
  exit 0
fi

emit_existing_content() {
  local path="$1"
  if [[ -f "$path" ]]; then
    cat "$path"
  elif [[ -n "$SOURCE_REF" ]] && git cat-file -e "$SOURCE_REF:$path" 2>/dev/null; then
    git show "$SOURCE_REF:$path"
  else
    printf ''
  fi
}

mkdir -p docs

total=${#DOCS[@]}

for idx in "${!DOCS[@]}"; do
  DOC_PATH="${DOCS[$idx]}"
  display_index=$((idx + 1))
  PROMPT_FILE="$TMP_DIR/${DOC_PATH//\//_}.prompt.md"

  {
    cat "$PROMPT_TEMPLATE"
    echo
    echo "---"
    echo "Target File: $DOC_PATH"
    echo "Existing Content:";
    emit_existing_content "$DOC_PATH"
    if [[ "$DRY_RUN" == "true" ]]; then
      echo
      echo "---"
      echo "Dry Run Instructions: Do not modify the repository. Provide a detailed plan for how you would rewrite this document, including section outlines, code/data flow references, and key points, but make no filesystem changes."
    fi
  } > "$PROMPT_FILE"

  if [[ -n "$PROMPT_ARCHIVE" ]]; then
    doc_dir=$(dirname "$DOC_PATH")
    mkdir -p "$PROMPT_ARCHIVE/$doc_dir"
    cp "$PROMPT_FILE" "$PROMPT_ARCHIVE/$DOC_PATH.prompt.md"
  fi

  CMD=(codex exec --full-auto --profile "$PROFILE" -c 'mcp_servers={}')
  if [[ -n "$CODEX_MODEL" ]]; then
    CMD+=(--model "$CODEX_MODEL")
  fi
  if [[ "$BYPASS_SANDBOX" == "true" ]]; then
    CMD+=(--dangerously-bypass-approvals-and-sandbox)
  fi

  echo "[codex] rewriting $DOC_PATH ($display_index/$total)"
  "${CMD[@]}" - < "$PROMPT_FILE"
done

echo
if [[ "$DRY_RUN" == "true" ]]; then
  echo "Run completed in dry-run mode; no files were changed. Re-run with DRY_RUN=false to apply edits."
else
  echo "Rewrite complete. Review changes with 'git status' and 'git diff'."
fi

if [[ -n "$PROMPT_ARCHIVE" ]]; then
  echo "Prompts archived under $PROMPT_ARCHIVE"
fi
