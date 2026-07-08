#!/usr/bin/env bash
set -euo pipefail

for name in NEGENTROPY_RELEASES_S3_AK NEGENTROPY_RELEASES_S3_SK NEGENTROPY_RELEASES_S3_BUCKET NEGENTROPY_RELEASES_S3_URL NEGENTROPY_RELEASES_PUBLIC_URL RELEASE_CHANNEL R2_ACCESS_PROBE_NAME RUNNER_TEMP GITHUB_RUN_ID GITHUB_SHA; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

probe_file="$RUNNER_TEMP/negentropy-r2-access.txt"
probe_key="$RELEASE_CHANNEL/.ci-access-check/$R2_ACCESS_PROBE_NAME.txt"
printf 'run=%s\nsha=%s\nchannel=%s\n' "$GITHUB_RUN_ID" "$GITHUB_SHA" "$RELEASE_CHANNEL" > "$probe_file"

AWS_ACCESS_KEY_ID="$NEGENTROPY_RELEASES_S3_AK" \
AWS_SECRET_ACCESS_KEY="$NEGENTROPY_RELEASES_S3_SK" \
AWS_DEFAULT_REGION=auto \
AWS_EC2_METADATA_DISABLED=true \
aws --endpoint-url "${NEGENTROPY_RELEASES_S3_URL%/}" s3api put-object \
  --bucket "$NEGENTROPY_RELEASES_S3_BUCKET" \
  --key "$probe_key" \
  --body "$probe_file" \
  --content-type "text/plain; charset=utf-8" \
  --cache-control "no-store" \
  --no-cli-pager >/dev/null
