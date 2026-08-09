#!/usr/bin/env bash
# Build the app image and push it to JFrog Artifactory (Docker local repo).
# Fill the placeholders below, or override the same names via the environment.
set -euo pipefail

# --- local config (placeholders) ---
ARTIFACTORY_DOCKER_REGISTRY="dm-registry.olivers-homelab2.cc"
ARTIFACTORY_USER="YOURUSER"
ARTIFACTORY_TOKEN="YOURPASS"
IMAGE_NAME="dm-photo-website"
TAG="v0.1.0"
PUSH_LATEST="1"
# -----------------------------------

die() {
  echo "error: $*" >&2
  exit 1
}

if [[ -z "${ARTIFACTORY_DOCKER_REGISTRY}" || "${ARTIFACTORY_DOCKER_REGISTRY}" == "artifactory.example.com/docker-local" ]]; then
  die "set ARTIFACTORY_DOCKER_REGISTRY at the top of this script (not the example placeholder)"
fi

if [[ -z "${TAG}" ]]; then
  TAG="$(git rev-parse --short HEAD)"
fi

ARTIFACTORY_PASSWORD="${ARTIFACTORY_TOKEN:-${ARTIFACTORY_PASSWORD}}"
if [[ -z "${ARTIFACTORY_USER}" || -z "${ARTIFACTORY_PASSWORD}" ]]; then
  die "set ARTIFACTORY_USER and ARTIFACTORY_TOKEN (or ARTIFACTORY_PASSWORD) at the top of this script"
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
