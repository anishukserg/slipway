#!/usr/bin/env bash
# Калитка коммита по решениям 8, 12 и 13: один набор шагов и машинный вердикт.
#
#   tools/gate.sh [<каталог дерева>]
#
# Хук pre-commit передаёт выгруженное дерево коммита; без аргумента
# проверяется рабочее дерево. Сборка идёт в target/gate* корня репозитория,
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
#   6. сборка всех целей с константными проверками реестров и тесты;
#   7. сверка кодов ошибок работает: пробная атака с неверным кодом падает,
#      с верным — проходит (решение 12);
#   8. атаки — doctest со сверкой кодов под RUSTC_BOOTSTRAP=1, в отдельном
#      каталоге сборки, с полом по числу прошедших;
#   9. документация без предупреждений и битых внутренних ссылок;
#  10. сборка всех целей на минимальной версии из rust-version;
#  11. проба сверки кодов и атаки на минимальной версии, с тем же полом;
#  12. зависимости — политика deny.toml по сохранённой базе уязвимостей, без
#      сети (решение 13).
#
# Отсутствующий инструмент — отказ шага, а не пропуск. Шаг без предмета
# проверки называется невыполненным, а не пройденным.
#
# Код возврата: 0 — пройдено; 1 — шаг упал; 2 — ошибка запуска.
# Последняя строка: `GATE OK (<n> из <m>; …)` или `GATE FAIL: <шаг>`.
set -uo pipefail
if locale -a 2>/dev/null | grep -qiE '^c\.utf-?8$'; then export LC_ALL=C.UTF-8; fi
# Отсутствующий тулчейн — отказ шага, а не скрытая загрузка внутри хука.
export RUSTUP_AUTO_INSTALL=0

# Пол только растёт: добавил атаку — подними; понижение — изменение правила.
DOCTEST_FLOOR=36

fail() { echo "GATE FAIL: $1"; exit "${2:-1}"; }

repo=$(git rev-parse --show-toplevel 2>/dev/null) || fail "не git-репозиторий" 2
git_dir=$(git rev-parse --absolute-git-dir)
tree=$(cd "${1:-$repo}" 2>/dev/null && pwd) || fail "нет каталога ${1:-}" 2
target="$repo/target/gate"
names="$git_dir/info/slipway-external-names"
total=12
passed=0
skipped=()
doctests=0
mkdir -p "$target" || fail "не создать $target" 2

# cargo_step <название> <журнал> <каталог сборки> <команда…>: запуск из дерева;
# при отказе — строки ошибок и хвост журнала, полный вывод остаётся в файле.
cargo_step() {
  local label=$1 log="$target/$2.log" build=$3
  shift 3
  (cd "$tree" && CARGO_TARGET_DIR="$build" "$@") > "$log" 2>&1
  local rc=$?
  if (( rc != 0 )); then
    grep -E '^((error|warning|bug)(\[[A-Za-z0-9_-]+\])?:|test .* FAILED$|---- |Diff in |Some expected error codes)' "$log" | head -n 40
    tail -n 12 "$log"
    echo "полный вывод: $log"
    fail "$label (код $rc)"
  fi
  passed=$((passed + 1))
}

# count_doctests <журнал>: число прошедших doctest в выводе cargo test.
count_doctests() {
  grep -cE '^test .+ - .+\(line [0-9]+\)( - compile fail)? \.\.\. ok$' "$1"
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

# Шаги 4–12 запускают cargo. Без манифеста в самом дереве cargo пошёл бы
# искать рабочее пространство в родительских каталогах и собрал бы чужой
# проект, поэтому манифест обязателен и передаётся явно.
[[ -f $tree/Cargo.toml ]] || fail "в дереве нет Cargo.toml — сборка не запускается" 2
manifest="$tree/Cargo.toml"

# 4. Форматирование.
cargo_step "cargo fmt --check" fmt "$target" cargo fmt --manifest-path "$manifest" --all --check

# 5. clippy.
cargo_step "cargo clippy -D warnings" clippy "$target" \
  cargo clippy --manifest-path "$manifest" --workspace --all-targets --locked -- -D warnings

# 6. Сборка всех целей и тесты. Doctest-атаки выполняются шагом 8.
cargo_step "cargo test --all-targets" test "$target" \
  cargo test --manifest-path "$manifest" --workspace --all-targets --no-fail-fast --locked

# 7. Сверка кодов ошибок работает. rustdoc сверяет коды compile_fail только в
# nightly-режиме; на stable его включает RUSTC_BOOTSTRAP=1 (решение 12). Без
# сверки атака прошла бы на любой ошибке компиляции, поэтому механизм
# проверяется до атак: неверный код обязан упасть, верный — пройти.
probe="$target/probe"
mkdir -p "$probe/src" || fail "не создать $probe" 2
# Файл перезаписывается только при изменении: иначе проба пересобиралась бы.
write_if_changed() {
  [[ -f $1 && $(< "$1") == "$2" ]] || printf '%s\n' "$2" > "$1"
}
write_if_changed "$probe/Cargo.toml" '[package]
name = "slipway-gate-probe"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]'
write_if_changed "$probe/src/lib.rs" '//! Проба сверки кодов ошибок в атаках (решение 12).
//!
//! Неверный код: тело даёт E0308, объявлен E0080 — обязана упасть.
//!
//! ```compile_fail,E0080
//! let _: u32 = "не число";
//! ```
//!
//! Верный код: то же тело — обязана пройти.
//!
//! ```compile_fail,E0308
//! let _: u32 = "не число";
//! ```'
# probe_codes <журнал> <каталог сборки> [+тулчейн]: отказ, если сверка не
# работает. Без явного тулчейна проба собирается тулчейном дерева — cargo
# запускается из него.
probe_codes() {
  local log="$target/$1.log" build=$2
  shift 2
  (cd "$tree" && CARGO_TARGET_DIR="$build" RUSTC_BOOTSTRAP=1 \
    cargo "$@" test --manifest-path "$probe/Cargo.toml" --doc) > "$log" 2>&1
  if ! grep -q 'Some expected error codes were not found' "$log" \
    || ! grep -qE '^test result: FAILED\. 1 passed; 1 failed;' "$log"; then
    tail -n 12 "$log"
    echo "полный вывод: $log"
    fail "сверка кодов ошибок в атаках не работает${1:+ на $1} — атаки прошли бы вакуумно (решение 12)"
  fi
}
probe_codes probe "$target/probe-build"
passed=$((passed + 1))

# 8. Атаки. Отдельный каталог сборки: RUSTC_BOOTSTRAP меняет отпечаток сборки
# зависимостей и сбрасывал бы кэш шагов 5, 6 и 9.
cargo_step "атаки: cargo test --doc" attacks "$target-attacks" \
  env RUSTC_BOOTSTRAP=1 cargo test --manifest-path "$manifest" --workspace --doc --no-fail-fast --locked
doctests=$(count_doctests "$target/attacks.log")
if (( doctests < DOCTEST_FLOOR )); then
  fail "прошло doctest $doctests при поле $DOCTEST_FLOOR — атаки не исполнялись или удалены"
fi

# 9. Документация.
cargo_step "cargo doc -D warnings" doc "$target" \
  env RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --workspace --no-deps --locked

# 10–11. Минимальная версия из rust-version рабочего пространства (решение 12):
# обещанная потребителям невыразимость проверяется на обещанном им компиляторе.
# Отсутствующий тулчейн этой версии — отказ шага.
msrv=$(sed -nE 's/^rust-version = "([0-9]+\.[0-9]+(\.[0-9]+)?)"$/\1/p' "$manifest" | head -n 1)
[[ -n $msrv ]] || fail "в Cargo.toml дерева нет rust-version — минимальная версия не проверяется" 2
[[ $msrv == *.*.* ]] || msrv="$msrv.0"

# 10. Сборка всех целей на минимальной версии.
cargo_step "cargo +$msrv check --all-targets" msrv-check "$target-msrv" \
  cargo "+$msrv" check --manifest-path "$manifest" --workspace --all-targets --locked

# 11. Проба сверки кодов и атаки на минимальной версии.
probe_codes msrv-probe "$target-msrv-probe" "+$msrv"
cargo_step "атаки на $msrv: cargo test --doc" msrv-attacks "$target-msrv-attacks" \
  env RUSTC_BOOTSTRAP=1 cargo "+$msrv" test --manifest-path "$manifest" --workspace --doc --no-fail-fast --locked
msrv_doctests=$(count_doctests "$target/msrv-attacks.log")
if (( msrv_doctests < DOCTEST_FLOOR )); then
  fail "на $msrv прошло doctest $msrv_doctests при поле $DOCTEST_FLOOR — атаки не исполнялись или удалены"
fi

# 12. Зависимости (решение 13): политика deny.toml по сохранённой базе
# уязвимостей, без сети — коммит не зависит от сети. Базу обновляет pre-push;
# без базы шаг отказывает, и она ставится командой `cargo deny fetch`.
[[ -f $tree/deny.toml ]] || fail "в дереве нет deny.toml — политика зависимостей не задана (решение 13)"
command -v cargo-deny > /dev/null || fail "cargo-deny не установлен — cargo install cargo-deny --locked" 2
cargo_step "cargo deny check" deny "$target" \
  cargo deny --manifest-path "$manifest" --config "$tree/deny.toml" --frozen check

verdict="GATE OK ($passed из $total; doctest $doctests, на $msrv — $msrv_doctests"
if (( ${#skipped[@]} )); then
  verdict+="; не выполнялось: ${skipped[*]}"
fi
echo "$verdict)"
