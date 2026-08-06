#!/usr/bin/env bash
# Build the app image and push it to JFrog Artifactory (Docker local repo).
# Artifactory itself is operated outside this repo — set registry/creds via env.
set -euo pipefail

die() {
  echo "error: $*" >&2
  exit 1
}

: "${ARTIFACTORY_DOCKER_REGISTRY:?ARTIFACTORY_DOCKER_REGISTRY is required (e.g. artifactory.example.com/docker-local)}"

IMAGE_NAME="${IMAGE_NAME:-dm-photo-website}"
TAG="${TAG:-$(git rev-parse --short HEAD)}"
PUSH_LATEST="${PUSH_LATEST:-0}"

ARTIFACTORY_PASSWORD="${ARTIFACTORY_TOKEN:-${ARTIFACTORY_PASSWORD:-}}"
if [[ -z "${ARTIFACTORY_USER:-}" || -z "${ARTIFACTORY_PASSWORD}" ]]; then
  die "ARTIFACTORY_USER and ARTIFACTORY_TOKEN (or ARTIFACTORY_PASSWORD) are required for docker login"
fi

# Login host is the registry hostname (before the first /).
LOGIN_HOST="${ARTIFACTORY_DOCKER_REGISTRY%%/*}"
FULL_IMAGE="${ARTIFACTORY_DOCKER_REGISTRY}/${IMAGE_NAME}"

echo "==> docker login ${LOGIN_HOST}"
echo "${ARTIFACTORY_PASSWORD}" | docker login "${LOGIN_HOST}" \
  --username "${ARTIFACTORY_USER}" \
  --password-stdin

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "==> docker build -t ${FULL_IMAGE}:${TAG}"
docker build -t "${FULL_IMAGE}:${TAG}" .

echo "==> docker push ${FULL_IMAGE}:${TAG}"
docker push "${FULL_IMAGE}:${TAG}"

if [[ "${PUSH_LATEST}" == "1" ]]; then
  echo "==> also tagging and pushing ${FULL_IMAGE}:latest"
  docker tag "${FULL_IMAGE}:${TAG}" "${FULL_IMAGE}:latest"
  docker push "${FULL_IMAGE}:latest"
fi

echo "==> done: ${FULL_IMAGE}:${TAG}"
