#!/usr/bin/env bash
set -euo pipefail

for name in NEGENTROPY_RELEASES_PUBLIC_URL RELEASE_CHANNEL RELEASE_VERSION R2_METADATA_URL RUNNER_TEMP; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

metadata="$RUNNER_TEMP/negentropy-release-metadata.json"
curl -fsSL "$R2_METADATA_URL?run=${GITHUB_RUN_ID:-local}" -o "$metadata"

public_url="${NEGENTROPY_RELEASES_PUBLIC_URL%/}"
jq -e \
  --arg channel "$RELEASE_CHANNEL" \
  --arg version "$RELEASE_VERSION" \
  --arg unix "$public_url/manage.sh" \
  --arg windows "$public_url/manage.ps1" \
  '
  (.channel == $channel)
  and (.releaseVersion == $version)
  and (.manage.unix == $unix)
  and (.manage.windows == $windows)
  and (if .channel == "beta"
        then (.betaVersion == $version)
          and (.baseVersion | (type == "string") and (length > 0))
          and (.betaNumber | type == "number")
          and (("v" + .baseVersion + "-beta." + (.betaNumber | tostring)) == $version)
        else true end)
  and (.artifacts | to_entries | all(.value.url | (type == "string") and (length > 0)))
  ' "$metadata" >/dev/null || {
  echo "metadata validation failed" >&2
  exit 1
}

for url in $(jq -r '(.artifacts[].url), .manage.unix, .manage.windows' "$metadata"); do
  curl -fsSI "$url" >/dev/null
done
