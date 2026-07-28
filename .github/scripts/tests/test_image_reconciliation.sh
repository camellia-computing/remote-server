#!/usr/bin/env bash
set -euo pipefail

root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT
fake_bin="$root/bin"
state="$root/state"
mkdir -p "$fake_bin" "$state"

cat > "$fake_bin/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

[[ "$1" == buildx && "$2" == imagetools ]]
operation="$3"
shift 3

resolve_digest() {
  local reference="$1"
  case "$reference" in
    "$FAKE_IMAGE@"sha256:*)
      printf '%s\n' "${reference#*@}"
      ;;
    "$FAKE_IMAGE:$FAKE_TAG")
      if [[ -f "$FAKE_STATE/version" ]]; then
        cat "$FAKE_STATE/version"
      fi
      ;;
    "$FAKE_IMAGE:sha-$FAKE_COMMIT")
      if [[ -f "$FAKE_STATE/commit" ]]; then
        cat "$FAKE_STATE/commit"
      fi
      ;;
    *)
      echo "unexpected reference: $reference" >&2
      exit 1
      ;;
  esac
}

case "$operation" in
  inspect)
    if [[ "${1:-}" == --raw ]]; then
      reference="$2"
      [[ "$reference" == "$FAKE_IMAGE@"sha256:* ]]
      if [[ "$FAKE_PLATFORMS" == valid ]]; then
        jq -nc '{
          manifests: [
            {platform: {os: "linux", architecture: "amd64"}},
            {platform: {os: "linux", architecture: "arm64"}},
            {platform: {os: "unknown", architecture: "unknown"}}
          ]
        }'
      else
        jq -nc '{
          manifests: [
            {platform: {os: "linux", architecture: "amd64"}}
          ]
        }'
      fi
      exit 0
    fi
    reference="$1"
    digest="$(resolve_digest "$reference")"
    if [[ -z "$digest" ]]; then
      echo 'manifest unknown' >&2
      exit 1
    fi
    printf 'Name: %s\nDigest: %s\n' "$reference" "$digest"
    ;;
  create)
    [[ "$1" == --tag ]]
    alias="$2"
    source="$3"
    digest="$(resolve_digest "$source")"
    [[ -n "$digest" ]]
    case "$alias" in
      "$FAKE_IMAGE:$FAKE_TAG")
        printf '%s\n' "$digest" > "$FAKE_STATE/version"
        ;;
      "$FAKE_IMAGE:sha-$FAKE_COMMIT")
        printf '%s\n' "$digest" > "$FAKE_STATE/commit"
        ;;
      *)
        exit 1
        ;;
    esac
    ;;
  *)
    exit 1
    ;;
esac
EOF

cat > "$fake_bin/cosign" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == verify ]]
[[ "$2" == "$FAKE_IMAGE@$FAKE_DIGEST" ]]
[[ "$3" == --certificate-identity && "$4" == "$FAKE_IDENTITY" ]]
[[ "$5" == --certificate-oidc-issuer && "$6" == "$FAKE_ISSUER" ]]
[[ "$FAKE_SIGNATURE" == valid ]]
EOF
chmod +x "$fake_bin/docker" "$fake_bin/cosign"

export FAKE_COMMIT=0123456789abcdef0123456789abcdef01234567
export FAKE_DIGEST=sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
export FAKE_IMAGE=ghcr.io/test/remote-server
export FAKE_ISSUER=https://token.actions.githubusercontent.com
export FAKE_PLATFORMS=valid
export FAKE_SIGNATURE=valid
export FAKE_STATE="$state"
export FAKE_TAG=v1.2.3
export FAKE_IDENTITY=https://github.com/test/remote-server/.github/workflows/release.yml@refs/heads/main

run_reconciler() {
  PATH="$fake_bin:$PATH" \
  GITHUB_DEFAULT_BRANCH=main \
  GITHUB_REPOSITORY=test/remote-server \
  IMAGE="$FAKE_IMAGE" \
  RELEASE_COMMIT="$FAKE_COMMIT" \
  RELEASE_OIDC_ISSUER="$FAKE_ISSUER" \
  RELEASE_SIGNING_IDENTITY="$FAKE_IDENTITY" \
  RELEASE_TAG="$FAKE_TAG" \
  RELEASE_VERSION=1.2.3 \
    bash .github/scripts/reconcile-verified-image.sh "$@"
}

output="$root/output"
GITHUB_OUTPUT="$output" run_reconciler discover >/dev/null
[[ "$(cat "$output")" == 'digest=' ]]

run_reconciler promote "$FAKE_DIGEST" >/dev/null
[[ "$(cat "$state/version")" == "$FAKE_DIGEST" ]]
[[ "$(cat "$state/commit")" == "$FAKE_DIGEST" ]]

: > "$output"
GITHUB_OUTPUT="$output" run_reconciler discover >/dev/null
[[ "$(cat "$output")" == "digest=$FAKE_DIGEST" ]]

rm "$state/commit"
run_reconciler promote "$FAKE_DIGEST" >/dev/null
[[ "$(cat "$state/commit")" == "$FAKE_DIGEST" ]]

other=sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
printf '%s\n' "$other" > "$state/commit"
if run_reconciler discover >/dev/null 2>&1; then
  echo 'Reconciliation accepted conflicting immutable aliases' >&2
  exit 1
fi
printf '%s\n' "$FAKE_DIGEST" > "$state/commit"

FAKE_SIGNATURE=invalid
export FAKE_SIGNATURE
if run_reconciler discover >/dev/null 2>&1; then
  echo 'Reconciliation accepted an invalid existing signature' >&2
  exit 1
fi
FAKE_SIGNATURE=valid
export FAKE_SIGNATURE

FAKE_PLATFORMS=invalid
export FAKE_PLATFORMS
if run_reconciler discover >/dev/null 2>&1; then
  echo 'Reconciliation accepted an incomplete platform set' >&2
  exit 1
fi

echo 'Verified image reconciliation tests passed'
