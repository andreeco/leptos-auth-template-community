#!/usr/bin/env bash
set -euo pipefail

# Generic deploy script for this template repo.
#
# What it does:
# 1) Build Docker image from local project
# 2) Push image to container registry
# 3) Optionally roll out remotely via ssh + docker compose
#
# Typical usage:
#   ./deploy.sh
#   ./deploy.sh --tag 0.1.0 --latest
#   ./deploy.sh --registry registry.example.com --image-repo myteam/leptos-login-template
#   ./deploy.sh --remote --server my-vps --remote-dir ~/apps/leptos-login-template
#
# Environment overrides (optional):
#   REGISTRY=ghcr.io
#   IMAGE_REPO=example/leptos-auth-template-community
#   TAG=20260101-120000
#   PROJECT_DIR=/path/to/repo
#   PUSH_LATEST=1|0
#   ROLL_OUT_REMOTE=1|0
#   SERVER=my-vps
#   REMOTE_DIR=~/apps/leptos-login-template
#   REMOTE_LOGIN=1|0
#   INTERACTIVE=1|0
#   PRECHECK=1|0
#   REGISTRY_USERNAME=...
#   REGISTRY_PASSWORD=...
#   COMPOSE_FILE=compose.yaml

log()  { printf '[deploy] %s\n' "$*"; }
warn() { printf '[deploy] WARN: %s\n' "$*" >&2; }
die()  { printf '[deploy] ERROR: %s\n' "$*" >&2; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

ask_yes_no() {
  local prompt="$1"
  local default="${2:-Y}" # Y or N
  local suffix="[y/N]"
  [[ "$default" == "Y" ]] && suffix="[Y/n]"

  while true; do
    read -r -p "$prompt $suffix " reply || return 1
    reply="${reply:-$default}"
    case "${reply,,}" in
      y|yes) return 0 ;;
      n|no)  return 1 ;;
      *) echo "Please answer yes or no." ;;
    esac
  done
}

ask_with_default() {
  local prompt="$1"
  local default="$2"
  local out
  read -r -p "$prompt [$default]: " out || true
  printf '%s' "${out:-$default}"
}

is_tty() {
  [[ -t 0 && -t 1 ]]
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# Defaults
REGISTRY="${REGISTRY:-ghcr.io}"
IMAGE_REPO="${IMAGE_REPO:-example/leptos-auth-template-community}"
TAG="${TAG:-$(date +%Y%m%d-%H%M%S)}"
PROJECT_DIR="${PROJECT_DIR:-$SCRIPT_DIR}"

PUSH_LATEST="${PUSH_LATEST:-1}"
ROLL_OUT_REMOTE="${ROLL_OUT_REMOTE:-0}"
SERVER="${SERVER:-}"
REMOTE_DIR="${REMOTE_DIR:-~/leptos-auth-template-community}"
COMPOSE_FILE="${COMPOSE_FILE:-compose.yaml}"

REMOTE_LOGIN="${REMOTE_LOGIN:-0}"
INTERACTIVE="${INTERACTIVE:-1}"
PRECHECK="${PRECHECK:-1}"

REGISTRY_USERNAME="${REGISTRY_USERNAME:-}"
REGISTRY_PASSWORD="${REGISTRY_PASSWORD:-}"

SHOW_HELP=0

usage() {
  cat <<'EOF'
Usage: ./deploy.sh [options]

Build/push:
  --registry <host>         Registry host (default: ghcr.io)
  --image-repo <name>       Repo path in registry (default: example/leptos-auth-template-community)
  --tag, -t <tag>           Image tag (default: timestamp)
  --latest                  Also tag/push :latest
  --no-latest               Do not push :latest
  --project-dir <path>      Project directory containing Dockerfile

Remote rollout:
  --remote                  Enable remote rollout
  --no-remote               Disable remote rollout
  --server <ssh-host>       SSH host for rollout
  --remote-dir <path>       Remote directory containing compose file
  --compose-file <file>     Compose file name in remote dir (default: compose.yaml)
  --remote-login            Perform non-interactive docker login on remote (requires credentials)

Behavior:
  --non-interactive         Disable all prompts
  --interactive             Enable prompts
  --no-precheck             Skip preflight checks
  --help, -h                Show this help

Environment variables may be used instead of flags:
  REGISTRY, IMAGE_REPO, TAG, PROJECT_DIR, PUSH_LATEST, ROLL_OUT_REMOTE,
  SERVER, REMOTE_DIR, COMPOSE_FILE, REMOTE_LOGIN, INTERACTIVE, PRECHECK,
  REGISTRY_USERNAME, REGISTRY_PASSWORD
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --registry)
      [[ $# -ge 2 ]] || die "--registry requires a value"
      REGISTRY="$2"; shift 2 ;;
    --image-repo)
      [[ $# -ge 2 ]] || die "--image-repo requires a value"
      IMAGE_REPO="$2"; shift 2 ;;
    --tag|-t)
      [[ $# -ge 2 ]] || die "--tag requires a value"
      TAG="$2"; shift 2 ;;
    --latest)
      PUSH_LATEST=1; shift ;;
    --no-latest)
      PUSH_LATEST=0; shift ;;
    --project-dir)
      [[ $# -ge 2 ]] || die "--project-dir requires a value"
      PROJECT_DIR="$2"; shift 2 ;;

    --remote)
      ROLL_OUT_REMOTE=1; shift ;;
    --no-remote)
      ROLL_OUT_REMOTE=0; shift ;;
    --server)
      [[ $# -ge 2 ]] || die "--server requires a value"
      SERVER="$2"; shift 2 ;;
    --remote-dir)
      [[ $# -ge 2 ]] || die "--remote-dir requires a value"
      REMOTE_DIR="$2"; shift 2 ;;
    --compose-file)
      [[ $# -ge 2 ]] || die "--compose-file requires a value"
      COMPOSE_FILE="$2"; shift 2 ;;
    --remote-login)
      REMOTE_LOGIN=1; shift ;;

    --interactive)
      INTERACTIVE=1; shift ;;
    --non-interactive)
      INTERACTIVE=0; shift ;;
    --no-precheck)
      PRECHECK=0; shift ;;

    --help|-h)
      SHOW_HELP=1; shift ;;
    *)
      die "unknown argument: $1" ;;
  esac
done

if [[ "$SHOW_HELP" == "1" ]]; then
  usage
  exit 0
fi

[[ -n "$REGISTRY" ]] || die "REGISTRY must not be empty"
[[ -n "$IMAGE_REPO" ]] || die "IMAGE_REPO must not be empty"
[[ -n "$TAG" ]] || die "TAG must not be empty"

IMAGE="${REGISTRY}/${IMAGE_REPO}:${TAG}"
IMAGE_LATEST="${REGISTRY}/${IMAGE_REPO}:latest"

require_cmd docker
if [[ "$ROLL_OUT_REMOTE" == "1" ]]; then
  require_cmd ssh
  [[ -n "$SERVER" ]] || die "--remote requires --server (or SERVER env)"
fi

[[ -d "$PROJECT_DIR" ]] || die "PROJECT_DIR does not exist: $PROJECT_DIR"
[[ -f "$PROJECT_DIR/Dockerfile" ]] || die "Dockerfile not found in: $PROJECT_DIR"

if [[ "$INTERACTIVE" == "1" ]] && is_tty; then
  log "Interactive confirmation"
  if ! ask_yes_no "Use registry '${REGISTRY}'?" "Y"; then
    REGISTRY="$(ask_with_default "Enter registry host" "$REGISTRY")"
  fi
  if ! ask_yes_no "Use image repo '${IMAGE_REPO}'?" "Y"; then
    IMAGE_REPO="$(ask_with_default "Enter image repo" "$IMAGE_REPO")"
  fi
  if ! ask_yes_no "Use tag '${TAG}'?" "Y"; then
    TAG="$(ask_with_default "Enter image tag" "$TAG")"
  fi

  IMAGE="${REGISTRY}/${IMAGE_REPO}:${TAG}"
  IMAGE_LATEST="${REGISTRY}/${IMAGE_REPO}:latest"

  if [[ "$ROLL_OUT_REMOTE" == "1" ]]; then
    if ! ask_yes_no "Roll out remotely to '${SERVER}:${REMOTE_DIR}'?" "Y"; then
      die "aborted by user"
    fi
  fi
fi

log "Registry:      $REGISTRY"
log "Image repo:    $IMAGE_REPO"
log "Tag:           $TAG"
log "Image:         $IMAGE"
log "Project dir:   $PROJECT_DIR"
log "Push latest:   $PUSH_LATEST"
log "Remote rollout:$ROLL_OUT_REMOTE"
if [[ "$ROLL_OUT_REMOTE" == "1" ]]; then
  log "Remote target: ${SERVER}:${REMOTE_DIR}"
  log "Compose file:  $COMPOSE_FILE"
fi

if [[ "$PRECHECK" == "1" ]]; then
  log "Running preflight checks"
  docker version >/dev/null 2>&1 || die "docker daemon is not reachable"

  if [[ "$ROLL_OUT_REMOTE" == "1" ]]; then
    ssh -o BatchMode=yes -o ConnectTimeout=8 "$SERVER" "echo ok >/dev/null" \
      || die "cannot reach remote host via ssh: $SERVER"
  fi

  if [[ "$INTERACTIVE" == "1" ]] && is_tty; then
    ask_yes_no "Proceed with build and push?" "Y" || die "aborted by user"
  fi
fi

# Local registry login
if [[ -n "$REGISTRY_USERNAME" && -n "$REGISTRY_PASSWORD" ]]; then
  log "Logging into registry (local) with provided credentials"
  printf '%s' "$REGISTRY_PASSWORD" | docker login "$REGISTRY" -u "$REGISTRY_USERNAME" --password-stdin
elif [[ "$INTERACTIVE" == "1" ]] && is_tty; then
  if ask_yes_no "Run local docker login for ${REGISTRY} now?" "Y"; then
    docker login "$REGISTRY"
  else
    log "Skipping local login; using existing docker credentials"
  fi
else
  log "No credentials provided; assuming existing local docker login session"
fi

log "Building image $IMAGE (refreshing base tags with --pull)"
DOCKER_BUILDKIT=1 docker build --pull -t "$IMAGE" "$PROJECT_DIR"

log "Pushing $IMAGE"
docker push "$IMAGE"

if [[ "$PUSH_LATEST" == "1" ]]; then
  log "Tagging/pushing latest: $IMAGE_LATEST"
  docker tag "$IMAGE" "$IMAGE_LATEST"
  docker push "$IMAGE_LATEST"
fi

DIGEST_LINE="$(docker inspect --format='{{index .RepoDigests 0}}' "$IMAGE" 2>/dev/null || true)"
if [[ -n "$DIGEST_LINE" ]]; then
  log "Pushed digest: $DIGEST_LINE"
fi

if [[ "$ROLL_OUT_REMOTE" == "1" ]]; then
  if [[ "$REMOTE_LOGIN" == "1" ]]; then
    [[ -n "$REGISTRY_USERNAME" && -n "$REGISTRY_PASSWORD" ]] || \
      die "--remote-login requires REGISTRY_USERNAME and REGISTRY_PASSWORD"
    log "Logging into registry on remote host (non-interactive)"
    printf '%s' "$REGISTRY_PASSWORD" | \
      ssh "$SERVER" "docker login '$REGISTRY' -u '$REGISTRY_USERNAME' --password-stdin"
  elif [[ "$INTERACTIVE" == "1" ]] && is_tty; then
    if ask_yes_no "Run remote docker login on ${SERVER} for ${REGISTRY} now?" "N"; then
      ssh -t "$SERVER" "docker login '$REGISTRY'"
    else
      log "Skipping remote docker login; using existing remote credentials"
    fi
  fi

  log "Rolling out remotely"
  ssh "$SERVER" bash -s -- "$REMOTE_DIR" "$COMPOSE_FILE" "$REGISTRY" "$IMAGE_REPO" "$TAG" "$IMAGE" <<'EOF'
set -euo pipefail

remote_dir="$1"
compose_file="$2"
registry="$3"
image_repo="$4"
tag="$5"
image="$6"

cd "$remote_dir"

if ! docker compose version >/dev/null 2>&1; then
  echo "[remote] ERROR: docker compose is not available" >&2
  exit 1
fi

if [ ! -f "$compose_file" ]; then
  echo "[remote] ERROR: compose file not found: $remote_dir/$compose_file" >&2
  exit 1
fi

# Export commonly used image variables for compose interpolation.
export IMAGE_REGISTRY="$registry"
export IMAGE_REPO="$image_repo"
export IMAGE_TAG="$tag"
export IMAGE="$image"

echo "[remote] Pulling latest image references"
docker compose -f "$compose_file" pull || true

echo "[remote] Recreating services"
docker compose -f "$compose_file" up -d --remove-orphans

echo "[remote] Status"
docker compose -f "$compose_file" ps
EOF
fi

log "Done"
log "Image: $IMAGE"
if [[ "$ROLL_OUT_REMOTE" == "1" ]]; then
  log "Remote rollout finished on $SERVER"
fi
