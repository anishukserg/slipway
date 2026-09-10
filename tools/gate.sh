#!/usr/bin/env bash
# Калитка коммита по решению 8: один набор шагов и машинный вердикт.
#
#   tools/gate.sh [<каталог дерева>]
#
# Хук pre-commit передаёт выгруженное дерево коммита; без аргумента
# проверяется рабочее дерево. Сборка идёт в target/gate корня репозитория,
# чтобы кэш переживал выгрузки; cargo запускается из самого дерева, чтобы
# тулчейн брался из его rust-toolchain.toml.
#
# Шаги, от дешёвых к дорогим:
#   1. внешние имена — по локальному списку <git-dir>/info/slipway-external-names
#      (решение 9); без списка шаг называется невыполненным, а не пройденным;
#   2. синтаксис скриптов правил — `bash -n` для tools/*.sh и .githooks/*;
#   3. относительные ссылки в markdown ведут в существующие файлы;
#   4. форматирование — `cargo fmt --check`;
#   5. clippy без предупреждений на всех целях;
#   6. cargo test — сборка всех целей с константными проверками реестров,
#      тесты, doctest-атаки с контролями;
#   7. пол числа прошедших doctest — атаки не могли пройти вакуумно;
#   8. документация без предупреждений и битых внутренних ссылок.
#
# Отсутствующий инструмент — отказ шага, а не пропуск. Шаг без предмета
# проверки называется невыполненным, а не пройденным.
#
# Код возврата: 0 — пройдено; 1 — шаг упал; 2 — ошибка запуска.
# Последняя строка: `GATE OK (<n> из <m>; …)` или `GATE FAIL: <шаг>`.
set -uo pipefail
if locale -a 2>/dev/null | grep -qiE '^c\.utf-?8$'; then export LC_ALL=C.UTF-8; fi

# Пол только растёт: добавил атаку — подними; понижение — изменение правила.
DOCTEST_FLOOR=36

fail() { echo "GATE FAIL: $1"; exit "${2:-1}"; }

repo=$(git rev-parse --show-toplevel 2>/dev/null) || fail "не git-репозиторий" 2
git_dir=$(git rev-parse --absolute-git-dir)
tree=$(cd "${1:-$repo}" 2>/dev/null && pwd) || fail "нет каталога ${1:-}" 2
target="$repo/target/gate"
names="$git_dir/info/slipway-external-names"
total=8
passed=0
skipped=()
doctests=0
mkdir -p "$target" || fail "не создать $target" 2

# cargo_step <название> <журнал> <команда…>: запуск из дерева; при отказе —
# строки ошибок и хвост журнала, полный вывод остаётся в файле.
cargo_step() {
  local label=$1 log="$target/$2.log"
  shift 2
  (cd "$tree" && CARGO_TARGET_DIR="$target" "$@") > "$log" 2>&1
  local rc=$?
  if (( rc != 0 )); then
    grep -E '^(error(\[E[0-9]+\])?:|warning:|test .* FAILED$|---- |Diff in )' "$log" | head -n 40
    tail -n 12 "$log"
    echo "полный вывод: $log"
    fail "$label (код $rc)"
  fi
  passed=$((passed + 1))
}

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

# 2. Синтаксис скриптов правил.
scripts=0
for script in "$tree"/tools/*.sh "$tree"/.githooks/*; do
  [[ -f $script ]] || continue
  scripts=$((scripts + 1))
  bash -n "$script" || fail "синтаксис ${script#"$tree"/}"
done
if (( scripts > 0 )); then
  passed=$((passed + 1))
else
  skipped+=("синтаксис скриптов")
fi

# 3. Относительные ссылки в markdown.
documents=0
broken=()
while IFS= read -r md; do
  documents=$((documents + 1))
  dir=$(dirname "$md")
  while IFS= read -r link; do
    path=${link%%#*}
    [[ -z $path || -e $dir/$path ]] || broken+=("${md#"$tree"/}: $link")
  done < <(grep -oE '\]\([^)[:space:]]+\)' "$md" | sed -E 's/^\]\((.*)\)$/\1/' | grep -vE '^(https?:|mailto:|#)')
# Каталоги отсекаются по имени внутри дерева, а не по подстроке пути: дерево
# коммита выгружается внутрь каталога git, и исключение «*/.git/*» отсекало
# все его файлы — шаг молча оставался без предмета.
done < <(find "$tree" \( -name target -o -name .git \) -prune -o -name '*.md' -print)
if (( ${#broken[@]} )); then
  printf '  битая ссылка: %s\n' "${broken[@]}"
  fail "относительные ссылки в markdown"
fi
if (( documents > 0 )); then
  passed=$((passed + 1))
else
  skipped+=("ссылки в markdown")
fi

# Шаги 4–8 запускают cargo. Без манифеста в самом дереве cargo пошёл бы
# искать рабочее пространство в родительских каталогах и собрал бы чужой
# проект, поэтому манифест обязателен и передаётся явно.
[[ -f $tree/Cargo.toml ]] || fail "в дереве нет Cargo.toml — сборка не запускается" 2
manifest="$tree/Cargo.toml"

# 4. Форматирование.
cargo_step "cargo fmt --check" fmt cargo fmt --manifest-path "$manifest" --all --check

# 5. clippy.
cargo_step "cargo clippy -D warnings" clippy \
  cargo clippy --manifest-path "$manifest" --workspace --all-targets --locked -- -D warnings

# 6. Сборка и тесты.
cargo_step "cargo test --workspace" test \
  cargo test --manifest-path "$manifest" --workspace --no-fail-fast --locked

# 7. Пол числа прошедших doctest.
doctests=$(grep -cE '^test .+ - .+\(line [0-9]+\)( - compile fail)? \.\.\. ok$' "$target/test.log")
if (( doctests < DOCTEST_FLOOR )); then
  fail "прошло doctest $doctests при поле $DOCTEST_FLOOR — атаки не исполнялись или удалены"
fi
passed=$((passed + 1))

# 8. Документация.
cargo_step "cargo doc -D warnings" doc \
  env RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --workspace --no-deps --locked

verdict="GATE OK ($passed из $total; doctest $doctests"
if (( ${#skipped[@]} )); then
  verdict+="; не выполнялось: ${skipped[*]}"
fi
echo "$verdict)"
