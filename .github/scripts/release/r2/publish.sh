#!/usr/bin/env bash
set -euo pipefail

for name in NEGENTROPY_RELEASES_S3_AK NEGENTROPY_RELEASES_S3_SK NEGENTROPY_RELEASES_S3_BUCKET NEGENTROPY_RELEASES_S3_URL NEGENTROPY_RELEASES_PUBLIC_URL RELEASE_CHANNEL RELEASE_VERSION RELEASE_ROOT GITHUB_OUTPUT GITHUB_REPOSITORY GITHUB_SHA GITHUB_RUN_ID GITHUB_RUN_ATTEMPT GITHUB_WORKFLOW; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

release_root="$RELEASE_ROOT"
public_url="${NEGENTROPY_RELEASES_PUBLIC_URL%/}"
version_prefix="$RELEASE_CHANNEL/versions/$RELEASE_VERSION"
latest_prefix="$RELEASE_CHANNEL/latest"
metadata_path="$release_root/metadata.json"

upload() {
  local file_path="$1"
  local object_key="$2"
  local content_type="$3"
  local cache_control="$4"
  if [ ! -f "$file_path" ]; then
    echo "expected upload file not found: $file_path" >&2
    exit 1
  fi
  AWS_ACCESS_KEY_ID="$NEGENTROPY_RELEASES_S3_AK" \
  AWS_SECRET_ACCESS_KEY="$NEGENTROPY_RELEASES_S3_SK" \
  AWS_DEFAULT_REGION=auto \
  AWS_EC2_METADATA_DISABLED=true \
  aws --endpoint-url "${NEGENTROPY_RELEASES_S3_URL%/}" s3api put-object \
    --bucket "$NEGENTROPY_RELEASES_S3_BUCKET" \
    --key "$object_key" \
    --body "$file_path" \
    --content-type "$content_type" \
    --cache-control "$cache_control" \
    --no-cli-pager >/dev/null
}

artifact_content_type() {
  case "$1" in
    *.tar.gz) printf '%s' "application/gzip" ;;
    *.zip) printf '%s' "application/zip" ;;
    *.json) printf '%s' "application/json; charset=utf-8" ;;
    *.txt) printf '%s' "text/plain; charset=utf-8" ;;
    *.sh) printf '%s' "text/x-shellscript; charset=utf-8" ;;
    *.ps1) printf '%s' "text/plain; charset=utf-8" ;;
    *) printf '%s' "application/octet-stream" ;;
  esac
}

for file_path in "$release_root"/negentropy-*.tar.gz "$release_root"/negentropy-*.zip "$release_root"/checksums.txt; do
  [ -f "$file_path" ] || continue
  name="$(basename "$file_path")"
  upload "$file_path" "$version_prefix/$name" "$(artifact_content_type "$name")" "public, max-age=31536000, immutable"
done

upload "$GITHUB_WORKSPACE/manage.sh" "manage.sh" "text/x-shellscript; charset=utf-8" "public, max-age=60, must-revalidate"
upload "$GITHUB_WORKSPACE/manage.ps1" "manage.ps1" "text/plain; charset=utf-8" "public, max-age=60, must-revalidate"

artifact_json() {
  local name="$1"
  local content_type="$2"
  local path="$release_root/$name"
  if [ ! -f "$path" ]; then
    echo "missing metadata source file: $path" >&2
    exit 1
  fi
  jq -n \
    --arg contentType "$content_type" \
    --arg name "$name" \
    --argjson size "$(stat -c %s "$path")" \
    --arg url "$public_url/$version_prefix/$name" \
    '{contentType: $contentType, name: $name, size: $size, url: $url}'
}

artifacts=$(jq -n \
  --argjson linuxX64 "$(artifact_json negentropy-x86_64-unknown-linux-gnu.tar.gz application/gzip)" \
  --argjson macArm64 "$(artifact_json negentropy-aarch64-apple-darwin.tar.gz application/gzip)" \
  --argjson macX64 "$(artifact_json negentropy-x86_64-apple-darwin.tar.gz application/gzip)" \
  --argjson winX64 "$(artifact_json negentropy-x86_64-pc-windows-msvc.zip application/zip)" \
  --argjson checksums "$(artifact_json checksums.txt 'text/plain; charset=utf-8')" \
  '{linuxX64: $linuxX64, macArm64: $macArm64, macX64: $macX64, winX64: $winX64, checksums: $checksums}')

metadata=$(jq -n \
  --arg channel "$RELEASE_CHANNEL" \
  --arg version "$RELEASE_VERSION" \
  --arg generated "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg repository "$GITHUB_REPOSITORY" \
  --arg commit "$GITHUB_SHA" \
  --argjson runId "$GITHUB_RUN_ID" \
  --argjson runAttempt "$GITHUB_RUN_ATTEMPT" \
  --arg workflow "$GITHUB_WORKFLOW" \
  --arg publicUrl "$public_url" \
  --arg latestMetadataUrl "$public_url/$latest_prefix/metadata.json" \
  --arg versionMetadataUrl "$public_url/$version_prefix/metadata.json" \
  --arg versionPrefix "$version_prefix" \
  --arg latestPrefix "$latest_prefix" \
  --arg manageUnix "$public_url/manage.sh" \
  --arg manageWindows "$public_url/manage.ps1" \
  --argjson artifacts "$artifacts" \
  '{
    version: 1,
    channel: $channel,
    releaseVersion: $version,
    generatedAt: $generated,
    github: {repository: $repository, commit: $commit, runId: $runId, runAttempt: $runAttempt, workflow: $workflow},
    r2: {publicUrl: $publicUrl, latestMetadataUrl: $latestMetadataUrl, versionMetadataUrl: $versionMetadataUrl, versionPrefix: $versionPrefix, latestPrefix: $latestPrefix},
    manage: {unix: $manageUnix, windows: $manageWindows},
    artifacts: $artifacts
  }')

guard_hash="${GUARD_VERSION_HASH:-}"
if [ -n "$guard_hash" ]; then
  metadata=$(printf '%s' "$metadata" | jq --arg hash "$guard_hash" '.guard = {version: {hash: $hash}}')
fi

state_source="${STATE_SOURCE:-workflow input}"
if [ "$RELEASE_CHANNEL" = "beta" ]; then
  if [[ ! "$RELEASE_VERSION" =~ ^v?([0-9]+\.[0-9]+\.[0-9]+)-beta\.([1-9][0-9]*)$ ]]; then
    echo "invalid beta release version: $RELEASE_VERSION" >&2
    exit 1
  fi
  match_base="${BASH_REMATCH[1]}"
  match_number="${BASH_REMATCH[2]}"
  base_version="${BASE_VERSION:-$match_base}"
  beta_number="${BETA_NUMBER:-$match_number}"
  if [ "$base_version" != "$match_base" ]; then
    echo "beta base mismatch: $base_version != $match_base" >&2
    exit 1
  fi
  if [ "$beta_number" != "$match_number" ]; then
    echo "beta number mismatch: $beta_number != $match_number" >&2
    exit 1
  fi
  metadata=$(printf '%s' "$metadata" | jq \
    --arg base "$base_version" \
    --argjson number "$beta_number" \
    --arg version "$RELEASE_VERSION" \
    --arg source "$state_source" \
    '.baseVersion = $base | .betaNumber = $number | .betaVersion = $version | .stateSource = $source')
else
  metadata=$(printf '%s' "$metadata" | jq \
    --arg version "$RELEASE_VERSION" \
    --arg source "$state_source" \
    '.stableVersion = $version | .stateSource = $source')
fi

printf '%s\n' "$metadata" >"$metadata_path"

upload "$metadata_path" "$version_prefix/metadata.json" "application/json; charset=utf-8" "public, max-age=31536000, immutable"
upload "$metadata_path" "$latest_prefix/metadata.json" "application/json; charset=utf-8" "public, max-age=60, must-revalidate"

{
  echo "metadata_url=$public_url/$latest_prefix/metadata.json"
  echo "version_metadata_url=$public_url/$version_prefix/metadata.json"
  echo "version_prefix=$version_prefix"
} >> "$GITHUB_OUTPUT"
