#!/bin/sh
set -eu

repo="DranxX/CodeMap"
version="${CODEMAP_VERSION:-}"

if [ -n "${CODEMAP_INSTALL_DIR:-}" ]; then
  install_dir="$CODEMAP_INSTALL_DIR"
else
  [ -n "${HOME:-}" ] || {
    printf 'codemap installer: HOME is not set; set CODEMAP_INSTALL_DIR\n' >&2
    exit 1
  }
  install_dir="${HOME}/.local/bin"
fi

fail() {
  printf 'codemap installer: %s\n' "$*" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v uname >/dev/null 2>&1 || fail "uname is required"
command -v mktemp >/dev/null 2>&1 || fail "mktemp is required"
[ -n "$install_dir" ] || fail 'HOME is not set; set CODEMAP_INSTALL_DIR'

case "$(uname -s)" in
  Linux) os="linux" ;;
  *) fail "unsupported operating system: $(uname -s)" ;;
esac

case "$(uname -m)" in
  x86_64 | amd64) arch="x86_64" ;;
  arm64 | aarch64) arch="aarch64" ;;
  *) fail "unsupported architecture: $(uname -m)" ;;
esac

asset="codemap-${os}-${arch}"
base_url="https://github.com/${repo}/releases"
if [ -n "$version" ]; then
  base_url="${base_url}/download/${version}"
else
  base_url="${base_url}/latest/download"
fi

mkdir -p "$install_dir" || fail "could not create install directory: ${install_dir}"
[ -d "$install_dir" ] && [ -w "$install_dir" ] || fail "install directory is not writable: ${install_dir}"
install_dir="$(cd "$install_dir" && pwd -P)"

tmp_file="$(mktemp "${install_dir}/.codemap.XXXXXX")" || fail 'could not create a temporary file'
trap 'rm -f "$tmp_file"' 0 HUP INT TERM

printf 'Downloading CodeMap %s for %s/%s...\n' "${version:-latest}" "$os" "$arch"
if ! curl -fL "${base_url}/${asset}" -o "$tmp_file"; then
  fail "download failed; check that release asset ${asset} exists"
fi

chmod 0755 "$tmp_file" || fail 'could not make the binary executable'
mv -f "$tmp_file" "${install_dir%/}/codemap" || fail 'could not install codemap'
trap - 0 HUP INT TERM

printf 'Installed codemap to %s\n' "${install_dir%/}/codemap"
case ":${PATH:-}:" in
  *":${install_dir}:"*) printf "Run 'codemap' to scan the current directory.\n" ;;
  *)
    printf 'Add this directory to PATH to run codemap from anywhere:\n  export PATH="%s:$PATH"\n' "$install_dir"
    ;;
esac
