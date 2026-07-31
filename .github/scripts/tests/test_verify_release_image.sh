#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "verify release image test: $*" >&2
  exit 1
}

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
verify_script="$script_directory/../verify-release-image.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/remote-image-readback.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
mkdir -p "$fixture/bin"

cat > "$fixture/bin/docker" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "$*" >> "$DOCKER_CALLS"
case "${1:-} ${2:-} ${3:-}" in
  "buildx imagetools inspect")
    printf '%s\n' "$DOCKER_INDEX_JSON"
    ;;
  "pull --platform linux/amd64"|"pull --platform linux/arm64")
    ;;
  "image inspect --format")
    format="${4:-}"
    reference="${5:-}"
    case "$format" in
      '{{.Id}}')
        case "$reference" in
          *@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)
            echo sha256:1111111111111111111111111111111111111111111111111111111111111111
            ;;
          *@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb)
            echo sha256:2222222222222222222222222222222222222222222222222222222222222222
            ;;
          *) exit 91 ;;
        esac
        ;;
      *org.opencontainers.image.version*) echo 1.0.0 ;;
      *org.opencontainers.image.revision*) echo dddddddddddddddddddddddddddddddddddddddd ;;
      *org.opencontainers.image.source*) echo https://github.com/example/remote-server ;;
      '{{.Config.User}}') echo 10001:10001 ;;
      *) exit 92 ;;
    esac
    ;;
  "tag "*)
    ;;
  "run --rm --platform")
    entrypoint=""
    while (($#)); do
      if [[ "$1" == --entrypoint ]]; then
        entrypoint="${2:-}"
        break
      fi
      shift
    done
    case "$entrypoint" in
      camellia-remote-identity) echo 'camellia-remote-identity 1.0.0' ;;
      camellia-remote-relay) echo 'camellia-remote-relay 1.0.0' ;;
      *) exit 93 ;;
    esac
    ;;
  *)
    echo "unexpected docker call: $*" >&2
    exit 94
    ;;
esac
SH
chmod +x "$fixture/bin/docker"

parent_digest="sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
amd64_digest="sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
arm64_digest="sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
repository="ghcr.io/example/remote-server"
export DOCKER_CALLS="$fixture/docker-calls"
docker_index_json="$(
  jq -nc --arg amd64 "$amd64_digest" --arg arm64 "$arm64_digest" '{
    schemaVersion: 2,
    mediaType: "application/vnd.oci.image.index.v1+json",
    manifests: [
      {mediaType:"application/vnd.oci.image.manifest.v1+json",digest:$amd64,platform:{os:"linux",architecture:"amd64"}},
      {mediaType:"application/vnd.oci.image.manifest.v1+json",digest:$arm64,platform:{os:"linux",architecture:"arm64"}}
    ]
  }'
)"
export DOCKER_INDEX_JSON="$docker_index_json"

PATH="$fixture/bin:$PATH" \
GITHUB_REPOSITORY=example/remote-server \
GITHUB_SERVER_URL=https://github.com \
  bash "$verify_script" "$repository@$parent_digest" 1.0.0 \
    dddddddddddddddddddddddddddddddddddddddd

grep -Fxq "pull --platform linux/amd64 $repository@$amd64_digest" "$DOCKER_CALLS" ||
  fail "amd64 was not pulled by its exact platform digest"
grep -Fxq "pull --platform linux/arm64 $repository@$arm64_digest" "$DOCKER_CALLS" ||
  fail "arm64 was not pulled by its exact platform digest"
if grep -Fq "pull --platform linux/amd64 $repository@$parent_digest" "$DOCKER_CALLS" ||
   grep -Fq "pull --platform linux/arm64 $repository@$parent_digest" "$DOCKER_CALLS"; then
  fail "the parent index digest was reused as a platform-local image"
fi

echo "Release image platform readback tests passed"
