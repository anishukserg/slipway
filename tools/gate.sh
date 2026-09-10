#!/usr/bin/env bash
# Калитка коммита по решению 8: один набор шагов и машинный вердикт.
#
#   tools/gate.sh [<каталог дерева>]
#
# Хук pre-commit передаёт выгруженное дерево коммита; без аргумента
# проверяется рабочее дерево. Сборка идёт в target/gate корня репозитория,
# чтобы кэш переживал выгрузки.
#
# Шаги:
#   1. внешние имена — по локальному списку <git-dir>/info/slipway-external-names
#      (решение 9); без списка шаг называется невыполненным, а не пройденным;
#   2. cargo test --workspace — сборка всех целей с константными проверками
#      реестров, тесты, doctest-атаки с контролями;
#   3. пол числа прошедших doctest — атаки не могли пройти вакуумно.
#
# Отсутствующий инструмент — отказ шага, а не пропуск. Форматирование и clippy
# не входят: в установленном тулчейне этих компонентов нет (решение 8, минусы).
#
# Код возврата: 0 — пройдено; 1 — шаг упал; 2 — ошибка запуска.
# Последняя строка: `GATE OK (<n> из <m>[; не выполнялось: …])` или `GATE FAIL: <шаг>`.
set -uo pipefail
if locale -a 2>/dev/null | grep -qiE '^c\.utf-?8$'; then export LC_ALL=C.UTF-8; fi

# Пол только растёт: добавил атаку — подними; понижение — изменение правила.
DOCTEST_FLOOR=36

fail() { echo "GATE FAIL: $1"; exit "${2:-1}"; }

repo=$(git rev-parse --show-toplevel 2>/dev/null) || fail "не git-репозиторий" 2
git_dir=$(git rev-parse --absolute-git-dir)
tree=$(cd "${1:-$repo}" 2>/dev/null && pwd) || fail "нет каталога ${1:-}" 2
target="$repo/target/gate"
log="$target/gate-test.log"
names="$git_dir/info/slipway-external-names"
total=3
passed=0
skipped=()

# 1. Внешние имена.
patterns=$(grep -vE '^[[:space:]]*(#|$)' "$names" 2>/dev/null)
if [[ -n $patterns ]]; then
  hits=$(grep -rIilF --exclude-dir=target --exclude-dir=.git -e "$patterns" "$tree")
  rc=$?
  case $rc in
    0) sed "s|^$tree/|  внешнее имя в файле: |" <<< "$hits"
       fail "внешние имена в дереве (решение 9)" ;;
    1) passed=$((passed + 1)) ;;
    *) fail "поиск внешних имён не выполнился (grep, код $rc)" 2 ;;
  esac
else
  echo "внешние имена: список $names пуст или не задан — шаг не выполнялся"
  skipped+=("внешние имена")
fi

# 2. Сборка и тесты на дереве. Без манифеста в самом дереве cargo пошёл бы
#    искать рабочее пространство в родительских каталогах и собрал бы чужой
#    проект, поэтому манифест обязателен и передаётся явно.
[[ -f $tree/Cargo.toml ]] || fail "в дереве нет Cargo.toml — сборка не запускается" 2
mkdir -p "$target" || fail "не создать $target" 2
CARGO_TARGET_DIR="$target" cargo test --manifest-path "$tree/Cargo.toml" --workspace --no-fail-fast > "$log" 2>&1
rc=$?
if (( rc != 0 )); then
  grep -E '^(error(\[E[0-9]+\])?:|test .* FAILED$|---- )' "$log" | head -n 40
  tail -n 12 "$log"
  echo "полный вывод: $log"
  fail "cargo test --workspace (код $rc)"
fi
passed=$((passed + 1))

# 3. Пол числа прошедших doctest.
doctests=$(grep -cE '^test .+ - .+\(line [0-9]+\)( - compile fail)? \.\.\. ok$' "$log")
if (( doctests < DOCTEST_FLOOR )); then
  fail "прошло doctest $doctests при поле $DOCTEST_FLOOR — атаки не исполнялись или удалены"
fi
passed=$((passed + 1))

verdict="GATE OK ($passed из $total; doctest $doctests"
if (( ${#skipped[@]} )); then
  verdict+="; не выполнялось: ${skipped[*]}"
fi
echo "$verdict)"
