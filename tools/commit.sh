#!/usr/bin/env bash
# Коммит по решению 8.
#
#   tools/commit.sh -F <файл сообщения> [--log <файл>] [--timeout <сек>] -- <пути…>
#
# Сообщение проверяется по форме до блокировки и хуков. Пути перечисляются
# явно — новые, изменённые и удалённые; коммитятся ровно они. Параллельный
# коммит ждёт блокировку, а не отказывает. С --log весь вывод хуков уходит
# в файл, на терминал — строки отказа и вердикт.
#
# Код возврата: 0 — коммит создан; 1 — по путям нечего коммитить или git add
# упал; 2 — неверные аргументы или окружение; 3 — блокировка не получена;
# 4 — коммит не создан: отказ проверки сообщения, хука или самого git.
# Последняя строка вывода: `COMMIT OK <sha>` или `COMMIT REFUSED: <причина>`.
set -uo pipefail

sha=""
reason=""
log=""
out=1

finish() {
  local rc=$?
  if [[ -n $log && -z $sha && -r $log ]]; then
    grep -E 'GATE FAIL|SELFTEST FAIL|отвергнуто|^  - |error(\[E[0-9]+\])?:|FAILED' "$log" | tail -n 40 >&"$out"
    echo "полный вывод: $log" >&"$out"
  fi
  if [[ -n $sha ]]; then
    echo "COMMIT OK $sha" >&"$out"
  else
    echo "COMMIT REFUSED: ${reason:-сбой (код $rc)}" >&"$out"
  fi
  exit "$rc"
}
trap finish EXIT
refuse() { reason=$2; exit "$1"; }

msg=""
timeout=600
while (( $# )); do
  case $1 in
    -F) (( $# >= 2 )) || refuse 2 "после -F нужен файл"; msg=$2; shift 2 ;;
    --log) (( $# >= 2 )) || refuse 2 "после --log нужен файл"; log=$2; shift 2 ;;
    --timeout) (( $# >= 2 )) || refuse 2 "после --timeout нужны секунды"; timeout=$2; shift 2 ;;
    --) shift; break ;;
    *) refuse 2 "неизвестный аргумент $1; пути перечисляются после --" ;;
  esac
done

[[ -n $msg && -r $msg ]] || refuse 2 "нужен -F <читаемый файл сообщения>"
msg=$(cd "$(dirname "$msg")" && pwd)/$(basename "$msg")
(( $# )) || refuse 2 "пути не перечислены: коммит всего изменённого запрещён"
command -v flock > /dev/null || refuse 2 "нет flock: без блокировки параллельный коммит заберёт чужой индекс"

root=$(git rev-parse --show-toplevel 2>/dev/null) || refuse 2 "не git-репозиторий"
cd "$root" || refuse 2 "нет доступа к $root"
git_dir=$(git rev-parse --absolute-git-dir)

if [[ -n $log ]]; then
  mkdir -p "$(dirname "$log")" || refuse 2 "не создать каталог для $log"
  exec 3>&1
  out=3
  exec > "$log" 2>&1
fi

bash tools/commit-msg-check.sh --form-only "$msg" || refuse 4 "сообщение не по форме (решение 8)"

exec 9> "$git_dir/slipway-commit.lock"
flock -w "$timeout" 9 || refuse 3 "блокировка коммита не получена за $timeout с"

git add -A -- "$@" || refuse 1 "git add упал на перечисленных путях"
if git diff --cached --quiet -- "$@"; then
  refuse 1 "по перечисленным путям нечего коммитить"
fi

if ! git commit --only -F "$msg" -- "$@"; then
  git reset -q -- "$@" 2> /dev/null
  refuse 4 "git commit не создал коммит: отказ хука или ошибка самого git (см. вывод)"
fi
sha=$(git rev-parse --short HEAD)
