#!/usr/bin/env bash
set -euo pipefail

reference="${1:?image@digest is required}"
version="${2:?stable version is required}"
revision="${3:?source revision is required}"

[[ "$reference" =~ @sha256:[0-9a-f]{64}$ ]]
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]
[[ "$revision" =~ ^[0-9a-f]{40}$ ]]

docker buildx imagetools inspect --raw "$reference" |
  jq -e '
    [
      .manifests[]? |
      select(.platform.os == "linux") |
      .platform.architecture
    ] | sort | unique == ["amd64", "arm64"]
  ' >/dev/null

for architecture in amd64 arm64; do
  docker pull --platform "linux/$architecture" "$reference"
  image_id="$(docker image inspect --format '{{.Id}}' "$reference")"
  local_tag="remote-server-release-verify:$architecture"
  docker tag "$image_id" "$local_tag"
  identity_version="$(docker run --rm --platform "linux/$architecture" \
    --entrypoint camellia-remote-identity "$local_tag" --version)"
  relay_version="$(docker run --rm --platform "linux/$architecture" \
    --entrypoint camellia-remote-relay "$local_tag" --version)"
  image_version="$(docker image inspect \
    --format '{{ index .Config.Labels "org.opencontainers.image.version" }}' \
    "$local_tag")"
  image_revision="$(docker image inspect \
    --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' \
    "$local_tag")"
  image_source="$(docker image inspect \
    --format '{{ index .Config.Labels "org.opencontainers.image.source" }}' \
    "$local_tag")"
  [[ "$identity_version" == "camellia-remote-identity $version" ]]
  [[ "$relay_version" == "camellia-remote-relay $version" ]]
  [[ "$image_version" == "$version" ]]
  [[ "$image_revision" == "$revision" ]]
  [[ "$image_source" == "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY" ]]
  [[ "$(docker image inspect --format '{{.Config.User}}' "$local_tag")" == "10001:10001" ]]
done
