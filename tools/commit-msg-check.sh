#!/usr/bin/env bash
# Проверка сообщения коммита по решению 8.
#
#   tools/commit-msg-check.sh [--form-only] <файл сообщения>
#
# Без --form-only дополнительно проверяется, что каждая единица работы из
# трейлера `Slipway-Work:` существует в дереве коммита (в индексе). Хук
# commit-msg вызывает полный вариант; tools/commit.sh — только форму и до
# блокировки, чтобы ошибка в теме не стоила прогона калитки.
#
# Код возврата: 0 — принято; 1 — отвергнуто (причины в stderr); 2 — ошибка запуска.
set -uo pipefail
if locale -a 2>/dev/null | grep -qiE '^c\.utf-?8$'; then export LC_ALL=C.UTF-8; fi

form_only=""
if [[ ${1:-} == --form-only ]]; then form_only=1; shift; fi
msg_file=${1:-}
if [[ -z $msg_file || ! -r $msg_file ]]; then
  echo "commit-msg-check: нужен читаемый файл сообщения" >&2
  exit 2
fi

types="FEAT FIX REFACTOR TEST DOCS ADR PLAN CHORE"
rule="правила коммитов — решение 8, crates/slipway-meta/src/adr/a0008.rs"
errors=()

# Области — значения оси подсистем из таксономии в индексе.
scopes=$(git show :crates/slipway-meta/src/taxonomy.rs 2>/dev/null \
  | sed -n 's/.*Subsystem *=> *\[\([^]]*\)\].*/\1/p' \
  | tr ',' '\n' | tr -d ' ' | tr '[:upper:]' '[:lower:]' | grep -v '^$')
if [[ -z $scopes ]]; then
  echo "commit-msg-check: в индексе нет значений оси Subsystem (crates/slipway-meta/src/taxonomy.rs)" >&2
  exit 2
fi

mapfile -t lines < <(grep -v '^#' "$msg_file")
subject=${lines[0]:-}

re='^\[([A-Z]+)\]\(([a-z0-9,]+)\): (.+)$'
if [[ $subject =~ $re ]]; then
  type=${BASH_REMATCH[1]}
  scope_list=${BASH_REMATCH[2]}
  summary=${BASH_REMATCH[3]}
  if [[ " $types " != *" $type "* ]]; then
    errors+=("тип [$type] не из набора: $types")
  fi
  IFS=',' read -r -a subject_scopes <<< "$scope_list"
  for scope in "${subject_scopes[@]}"; do
    if ! grep -qx -- "$scope" <<< "$scopes"; then
      errors+=("область ($scope) не значение оси подсистем: $(tr '\n' ' ' <<< "$scopes")")
    fi
  done
  if [[ $summary == *. ]]; then
    errors+=("точка в конце темы")
  fi
  if (( ${#subject} > 72 )); then
    errors+=("тема длиннее 72 символов (${#subject})")
  fi
else
  errors+=("тема не по форме [ТИП](область): суть — «$subject»")
fi

if (( ${#lines[@]} > 1 )) && [[ -n ${lines[1]} ]]; then
  errors+=("после темы нужна пустая строка")
fi

if grep -E '^Slipway-Work:' "$msg_file" | grep -vqE '^Slipway-Work: w[0-9]{4}$'; then
  errors+=("трейлер Slipway-Work не по форме wNNNN")
fi
mapfile -t works < <(grep -E '^Slipway-Work: w[0-9]{4}$' "$msg_file" | sed 's/^Slipway-Work: //')
if (( ${#works[@]} == 0 )); then
  errors+=("нет трейлера Slipway-Work: wNNNN — у коммита нет основания в плане")
elif [[ -z $form_only ]]; then
  for work in "${works[@]}"; do
    if ! git cat-file -e ":crates/slipway-plan/src/work/$work.rs" 2>/dev/null; then
      errors+=("единицы работы $work нет в дереве коммита (crates/slipway-plan/src/work/$work.rs)")
    fi
  done
fi

if (( ${#errors[@]} )); then
  echo "commit-msg-check: сообщение отвергнуто ($rule):" >&2
  printf '  - %s\n' "${errors[@]}" >&2
  exit 1
fi
