#!/usr/bin/env bash
# Самотест правил коммитов (решение 8): каждая проверка обязана отвергать то,
# против чего написана, а корректный коммит — проходить ровно с перечисленными
# путями. Работает во временном репозитории и не трогает текущий.
#
#   tools/test-commit-rules.sh
#
# Временный репозиторий создаётся в каталоге git текущего репозитория, а не
# в TMPDIR: TMPDIR может указывать внутрь чужого проекта.
#
# Последняя строка: `SELFTEST OK (<n> проверок)` или `SELFTEST FAIL: <проверка>`.
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
host_git_dir=$(git -C "$here" rev-parse --absolute-git-dir 2>/dev/null) \
  || { echo "SELFTEST FAIL: самотест запущен вне git-репозитория"; exit 1; }
# Хук передаёт git-командам переменные своего репозитория (GIT_INDEX_FILE и
# другие). Во временном репозитории они направили бы запись в индекс коммита,
# ради которого запущен хук, поэтому снимаются до первой команды там.
# shellcheck disable=SC2046
unset $(git rev-parse --local-env-vars)
tmp=$(mktemp -d "$host_git_dir/slipway-selftest.XXXXXX") \
  || { echo "SELFTEST FAIL: не создан временный каталог в $host_git_dir"; exit 1; }
trap 'rm -rf "$tmp"' EXIT
checks=0
fail() { echo "SELFTEST FAIL: $1"; exit 1; }

repo="$tmp/repo"
mkdir -p "$repo/tools" "$repo/hooks" "$repo/crates/slipway-meta" \
  "$repo/crates/slipway-plan/work" "$repo/old"
cp "$here/commit-msg-check.sh" "$here/commit.sh" "$here/gate.sh" "$repo/tools/"
printf 'slipway_core::declare_taxonomy! {\n    Subsystem => [Knowledge, Cli],\n}\n' \
  > "$repo/crates/slipway-meta/taxonomy.rs"
echo 'work' > "$repo/crates/slipway-plan/work/w0001.rs"
echo 'old' > "$repo/old/file.txt"
echo 'unrelated' > "$repo/unrelated.txt"
# Во временном репозитории подключена только проверка сообщения: калитка
# проверяется отдельно ниже, на своих шагах.
printf '#!/usr/bin/env bash\nexec bash "$(git rev-parse --show-toplevel)/tools/commit-msg-check.sh" "$1"\n' \
  > "$repo/hooks/commit-msg"
chmod +x "$repo/hooks/commit-msg"
printf '[CHORE](cli): база\n\nSlipway-Work: w0001\n' > "$tmp/base-msg"
(
  cd "$repo" &&
  git init -q &&
  git config user.name selftest &&
  git config user.email selftest@localhost &&
  git config core.hooksPath hooks &&
  git add -A &&
  git commit -q -F "$tmp/base-msg"
) > /dev/null 2>&1 || fail "не создан временный репозиторий с корректным базовым коммитом"

# attempt <ожидаемый код> <название> <сообщение> [пути…]
attempt() {
  local want=$1 label=$2 text=$3
  shift 3
  printf '%s\n' "$text" > "$tmp/msg"
  local output rc last
  output=$(cd "$repo" && bash tools/commit.sh -F "$tmp/msg" -- "$@" 2>&1)
  rc=$?
  checks=$((checks + 1))
  last=$(tail -n 1 <<< "$output")
  if [[ $rc != "$want" ]]; then
    tail -n 8 <<< "$output"
    fail "$label: код $rc, ожидался $want"
  fi
  if [[ $want == 0 && $last != "COMMIT OK "* ]] || [[ $want != 0 && $last != "COMMIT REFUSED: "* ]]; then
    fail "$label: последняя строка «$last»"
  fi
}

ok=$'[FEAT](cli): новый файл и удаление старого\n\nSlipway-Work: w0001'
echo 'new' > "$repo/new.txt"
attempt 4 "тема без типа" $'новый файл\n\nSlipway-Work: w0001' new.txt
attempt 4 "тип вне набора" $'[FEATURE](cli): новый файл\n\nSlipway-Work: w0001' new.txt
attempt 4 "область вне таксономии" $'[FEAT](wal): новый файл\n\nSlipway-Work: w0001' new.txt
attempt 4 "точка в конце темы" $'[FEAT](cli): новый файл.\n\nSlipway-Work: w0001' new.txt
attempt 4 "без основания" '[FEAT](cli): новый файл' new.txt
attempt 4 "основание вне плана" $'[FEAT](cli): новый файл\n\nSlipway-Work: w0099' new.txt
attempt 2 "без путей" "$ok"
attempt 1 "нечего коммитить" "$ok" crates

echo 'changed' >> "$repo/unrelated.txt"
rm -r "$repo/old"
attempt 0 "корректный коммит" "$ok" new.txt old
changed=$(cd "$repo" && git show --name-status --format= HEAD | sort | tr '\t\n' ': ')
checks=$((checks + 1))
[[ $changed == "A:new.txt D:old/file.txt " ]] || fail "коммит содержит не ровно перечисленные пути: «$changed»"
checks=$((checks + 1))
if (cd "$repo" && git diff --quiet -- unrelated.txt); then
  fail "чужое изменение unrelated.txt пропало из рабочего дерева"
fi

# Калитка: внешнее имя в дереве отвергается до сборки.
echo 'zzvneshniy' > "$repo/.git/info/slipway-external-names"
echo 'текст с именем ZZVneshniy внутри' > "$repo/leak.txt"
output=$(cd "$repo" && bash tools/gate.sh 2>&1)
rc=$?
checks=$((checks + 1))
if [[ $rc != 1 || $(tail -n 1 <<< "$output") != "GATE FAIL: внешние имена"* ]]; then
  tail -n 5 <<< "$output"
  fail "калитка пропустила внешнее имя (код $rc)"
fi

# Контроль: без списка шаг назван невыполненным, а не пройденным.
rm "$repo/.git/info/slipway-external-names" "$repo/leak.txt"
output=$(cd "$repo" && bash tools/gate.sh 2>&1)
checks=$((checks + 1))
grep -q "шаг не выполнялся" <<< "$output" || fail "без списка внешних имён шаг выглядит пройденным"

# Без манифеста в дереве калитка не запускает cargo: иначе он нашёл бы рабочее
# пространство в родительском каталоге и собрал бы чужой проект.
checks=$((checks + 1))
[[ $(tail -n 1 <<< "$output") == "GATE FAIL: в дереве нет Cargo.toml"* ]] \
  || fail "калитка запустила сборку без манифеста в дереве: «$(tail -n 1 <<< "$output")»"

echo "SELFTEST OK ($checks проверок)"
