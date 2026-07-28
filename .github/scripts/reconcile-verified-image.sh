#!/usr/bin/env bash
set -euo pipefail

: "${IMAGE:?IMAGE is required}"
: "${RELEASE_COMMIT:?RELEASE_COMMIT is required}"
: "${RELEASE_OIDC_ISSUER:?RELEASE_OIDC_ISSUER is required}"
: "${RELEASE_SIGNING_IDENTITY:?RELEASE_SIGNING_IDENTITY is required}"
: "${RELEASE_TAG:?RELEASE_TAG is required}"
: "${RELEASE_VERSION:?RELEASE_VERSION is required}"

[[ "$IMAGE" =~ ^ghcr\.io/[a-z0-9]+([._-][a-z0-9]+)*/[a-z0-9]+([._-][a-z0-9]+)*$ ]] || {
  echo 'IMAGE must be a lowercase GHCR repository' >&2
  exit 2
}
[[ "$RELEASE_COMMIT" =~ ^[0-9a-f]{40}$ ]] || {
  echo 'RELEASE_COMMIT must be a full commit SHA' >&2
  exit 2
}
[[ "$RELEASE_TAG" =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || {
  echo 'RELEASE_TAG must be canonical stable SemVer' >&2
  exit 2
}
[[ "$RELEASE_TAG" == "v$RELEASE_VERSION" ]] || {
  echo 'RELEASE_VERSION does not match RELEASE_TAG' >&2
  exit 2
}
[[ "$RELEASE_OIDC_ISSUER" == 'https://token.actions.githubusercontent.com' ]] || {
  echo 'RELEASE_OIDC_ISSUER must be the GitHub Actions issuer' >&2
  exit 2
}
[[ "$RELEASE_SIGNING_IDENTITY" == \
  "https://github.com/${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}/.github/workflows/release.yml@refs/heads/${GITHUB_DEFAULT_BRANCH:?GITHUB_DEFAULT_BRANCH is required}" ]] || {
  echo 'RELEASE_SIGNING_IDENTITY is not the trusted default-branch workflow' >&2
  exit 2
}

aliases=(
  "$IMAGE:$RELEASE_TAG"
  "$IMAGE:sha-$RELEASE_COMMIT"
)

digest_for() {
  local digest output reference="$1"
  if ! output="$(docker buildx imagetools inspect "$reference" 2>&1)"; then
    if grep -Eqi 'manifest unknown|no such manifest|not found' <<< "$output"; then
      return 0
    fi
    echo "Unable to inspect image reference $reference: $output" >&2
    return 1
  fi
  digest="$(awk '/^Digest:/ { print $2; exit }' <<< "$output")"
  [[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] || {
    echo "Registry returned no canonical digest for $reference" >&2
    return 1
  }
  printf '%s\n' "$digest"
}

verify_platforms() {
  local digest="$1" manifest
  manifest="$(docker buildx imagetools inspect --raw "$IMAGE@$digest")"
  jq -e '
    [.manifests[]? |
      select(.platform.os == "linux") |
      .platform.architecture
    ] | sort | unique == ["amd64", "arm64"]
  ' <<< "$manifest" >/dev/null || {
    echo "Image $IMAGE@$digest does not contain exactly the required Linux architectures" >&2
    return 1
  }
}

verify_signature() {
  local digest="$1"
  cosign verify "$IMAGE@$digest" \
    --certificate-identity "$RELEASE_SIGNING_IDENTITY" \
    --certificate-oidc-issuer "$RELEASE_OIDC_ISSUER" >/dev/null
}

verify_digest() {
  local canonical digest="$1"
  [[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] || {
    echo "Invalid image digest: $digest" >&2
    return 1
  }
  canonical="$(digest_for "$IMAGE@$digest")"
  [[ "$canonical" == "$digest" ]] || {
    echo "Image digest is unavailable from GHCR: $IMAGE@$digest" >&2
    return 1
  }
  verify_platforms "$digest"
  verify_signature "$digest"
}

write_output() {
  local digest="$1"
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    printf 'digest=%s\n' "$digest" >> "$GITHUB_OUTPUT"
  fi
  printf '%s\n' "$digest"
}

discover() {
  local candidate digest=""
  for alias in "${aliases[@]}"; do
    candidate="$(digest_for "$alias")"
    if [[ -n "$candidate" ]]; then
      if [[ -n "$digest" && "$candidate" != "$digest" ]]; then
        echo 'Immutable image aliases resolve to different digests' >&2
        return 1
      fi
      digest="$candidate"
    fi
  done
  if [[ -n "$digest" ]]; then
    verify_digest "$digest"
  fi
  write_output "$digest"
}

promote() {
  local current digest="$1"
  verify_digest "$digest"
  for alias in "${aliases[@]}"; do
    current="$(digest_for "$alias")"
    if [[ -z "$current" ]]; then
      docker buildx imagetools create --tag "$alias" "$IMAGE@$digest"
      current="$(digest_for "$alias")"
    fi
    [[ "$current" == "$digest" ]] || {
      echo "Immutable image alias conflict: $alias resolves to $current, expected $digest" >&2
      return 1
    }
  done
  verify_digest "$digest"
  write_output "$digest"
}

case "${1:-}" in
  discover)
    [[ "$#" == 1 ]] || {
      echo 'discover accepts no arguments' >&2
      exit 2
    }
    discover
    ;;
  promote)
    [[ "$#" == 2 ]] || {
      echo 'promote requires one digest argument' >&2
      exit 2
    }
    promote "$2"
    ;;
  *)
    echo 'Usage: reconcile-verified-image.sh discover | promote <sha256:digest>' >&2
    exit 2
    ;;
esac
