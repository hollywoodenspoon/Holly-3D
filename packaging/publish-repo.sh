#!/usr/bin/env bash
# Builds MiniMax with makepkg and publishes it to a local pacman repository,
# so `sudo pacman -Syu` installs and upgrades it like any other package.
#
#   packaging/publish-repo.sh             # build + publish
#   packaging/publish-repo.sh --nocheck   # extra flags go to makepkg
#
# Repository folder: ~/.local/share/minimax-repo (override with MINIMAX_REPO_DIR).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="${MINIMAX_REPO_DIR:-$HOME/.local/share/minimax-repo}"
db="$repo_dir/minimax.db.tar.gz"

for tool in makepkg repo-add git; do
  command -v "$tool" >/dev/null || { echo "error: '$tool' not found (sudo pacman -S --needed base-devel pacman git)" >&2; exit 1; }
done
if [[ $EUID -eq 0 ]]; then
  echo "error: run this as your normal user, not root (makepkg refuses to run as root)" >&2
  exit 1
fi

cd "$here"
# makepkg writes the new pkgver into PKGBUILD; put the committed file back
# afterwards so the git checkout stays clean.
backup="$(mktemp)"
cp PKGBUILD "$backup"
trap 'cp "$backup" PKGBUILD; rm -f "$backup"' EXIT

echo "==> Building MiniMax (makepkg -sf $*)"
makepkg -sf "$@"

# Full paths of the packages just built (honours PKGDEST from makepkg.conf).
mapfile -t built < <(makepkg --packagelist)
pkgs=()
for p in "${built[@]}"; do
  [[ -f $p ]] && pkgs+=("$p")
done
if [[ ${#pkgs[@]} -eq 0 ]]; then
  echo "error: makepkg finished but no package file was found" >&2
  exit 1
fi

echo "==> Publishing to $repo_dir"
mkdir -p "$repo_dir"
chmod 755 "$repo_dir"
added=()
for p in "${pkgs[@]}"; do
  install -m644 "$p" "$repo_dir/"
  added+=("$repo_dir/$(basename "$p")")
done
# -R: delete the package file of the version this one replaces.
repo-add -R "$db" "${added[@]}"
chmod 644 "$repo_dir"/minimax.db* "$repo_dir"/minimax.files* 2>/dev/null || true

cat <<MSG

==> Done: $(basename "${added[0]}")

Add these 3 lines to the end of /etc/pacman.conf (once):

[minimax]
SigLevel = Optional TrustAll
Server = file://$repo_dir

Then install or upgrade with:

  sudo pacman -Syu minimax

MSG

# pacman 7 downloads as the 'alpm' user (DownloadUser in pacman.conf), which
# must be able to walk into the repository folder.
if grep -Eq '^[[:space:]]*DownloadUser' /etc/pacman.conf 2>/dev/null; then
  blocked=()
  d="$repo_dir"
  while [[ $d != / ]]; do
    [[ $(stat -c %A "$d") == *[xt] ]] || blocked+=("$d")
    d="$(dirname "$d")"
  done
  if [[ ${#blocked[@]} -gt 0 ]]; then
    cat <<WARN
Warning: pacman downloads as the 'alpm' user (DownloadUser in /etc/pacman.conf),
which cannot enter these folders:
$(printf '  %s\n' "${blocked[@]}")
'sudo pacman -Syu' will then fail to read the [minimax] repository. Fix it with one of:
  chmod o+x $(printf '%q ' "${blocked[@]}")  # traverse only, no listing
  MINIMAX_REPO_DIR=/path/readable/by/alpm $0  # publish somewhere else
WARN
  fi
fi
