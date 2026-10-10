#!/usr/bin/env bash
set -euo pipefail
[[ ${DEV_DEPLOY_ENABLED:-} == true ]] || { echo 'Dev deploy is disabled'; exit 1; }
[[ ${DEV_DEPLOY_TARGET_CONFIRMED:-} == unity-games.wos.ktsys.jp ]] || { echo 'Target not confirmed'; exit 1; }
[[ ${SOURCE_SHA:-} =~ ^[a-f0-9]{40}$ ]] || exit 1
[[ ${DEV_DEPLOY_HOST:-} =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]*$ ]] || exit 1
[[ ${DEV_DEPLOY_USER:-} =~ ^[a-z_][a-z0-9_-]*$ ]] || exit 1
[[ ${DEV_DEPLOY_PORT:-} =~ ^[0-9]+$ ]] || exit 1
(( DEV_DEPLOY_PORT > 0 && DEV_DEPLOY_PORT < 65536 )) || exit 1
[[ ${DEV_DEPLOY_REPO_PATH:-} =~ ^/[a-zA-Z0-9_./-]+$ && $DEV_DEPLOY_REPO_PATH != *'/../'* && $DEV_DEPLOY_REPO_PATH != / ]] || exit 1
[[ -n ${DEV_DEPLOY_KNOWN_HOSTS:-} && -n ${DEV_DEPLOY_SSH_KEY:-} ]] || { echo 'Approved credentials and pinned known_hosts required'; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
chmod 700 "$work"
printf '%s\n' "$DEV_DEPLOY_SSH_KEY" > "$work/key"
printf '%s\n' "$DEV_DEPLOY_KNOWN_HOSTS" > "$work/known_hosts"
chmod 600 "$work/key" "$work/known_hosts"
unset DEV_DEPLOY_SSH_KEY
ssh -i "$work/key" -p "$DEV_DEPLOY_PORT" -o BatchMode=yes -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$work/known_hosts" -o ConnectTimeout=20 \
  "$DEV_DEPLOY_USER@$DEV_DEPLOY_HOST" "bash -s -- '$DEV_DEPLOY_REPO_PATH' '$SOURCE_SHA'" <<'REMOTE'
set -euo pipefail
repo=$1
sha=$2
cd "$repo"
[[ $(git rev-parse --show-toplevel) == "$repo" ]] || { echo 'Not exact repository root'; exit 1; }
[[ $(git branch --show-current) == dev ]] || { echo 'Not on dev'; exit 1; }
case $(git remote get-url origin) in
 https://github.com/ssnshino/rust-sandbox|https://github.com/ssnshino/rust-sandbox.git|git@github.com:ssnshino/rust-sandbox.git) ;;
 *) echo 'Unexpected origin'; exit 1;;
esac
exec 9>"$(git rev-parse --git-dir)/unity-games-deploy.lock"
flock -n 9 || { echo 'Concurrent server deploy'; exit 1; }
[[ -z $(git status --porcelain --untracked-files=normal) ]] || { echo 'Working tree is not clean'; exit 1; }
git fetch origin dev
[[ $(git rev-parse origin/dev) == "$sha" ]] || { echo 'Stale run: latest dev differs'; exit 1; }
git merge-base --is-ancestor HEAD "$sha" || { echo 'Non-fast-forward'; exit 1; }
# Validate proposed configuration before changing the bind-mounted live checkout.
staged=$(mktemp "$(git rev-parse --git-dir)/unity-games-compose.XXXXXX")
trap 'rm -f "$staged"' EXIT
git show "$sha:compose.dev.yaml" > "$staged"
config=$(docker compose --project-directory "$repo" -f "$staged" config --format json)
printf '%s' "$config" | python3 -c 'import json,sys,os
c=json.load(sys.stdin); s=c["services"]["airrace-web"]; e=s["environment"]; root=sys.argv[1]
assert s["container_name"]=="ayano-lab-unity-rust-games-web"
assert set(e["VIRTUAL_HOST"].split(","))=={"unity-games.wos.ktsys.jp","airrace-web.wos.ktsys.jp"}
assert str(e["VIRTUAL_PORT"])=="3000"
assert s["build"]["context"]==root and s["build"]["dockerfile"]=="web/Dockerfile"
assert s["working_dir"]=="/workspace/web"
assert s["command"]==["sh","-lc","npm run build && npm run start"]
assert not any(s.get(k) for k in ["privileged","devices","cap_add","network_mode","pid","ipc"])
ports=s["ports"]; assert len(ports)==1 and str(ports[0]["published"])=="18093" and str(ports[0]["target"])=="3000" and ports[0].get("protocol","tcp")=="tcp"
vols=s["volumes"]; assert len(vols)==3
expected={"/workspace/web":(root+"/web",False),"/workspace/app/src/airrace":(root+"/app/src/airrace",True)}
for v in vols:
 if v["target"]=="/workspace/web/node_modules": assert v["type"]=="volume" and not v.get("source")
 else:
  source,ro=expected[v["target"]]; assert v["type"]=="bind" and v["source"]==source and bool(v.get("read_only",False))==ro
assert set(s["networks"])=={"default","wos-proxy-network"}
assert c["networks"]["wos-proxy-network"]["external"] is True' "$repo"
previous=$(git rev-parse HEAD)
echo "Previous SHA: $previous; requested SHA: $sha"
git merge --ff-only "$sha"
[[ $(git rev-parse HEAD) == "$sha" ]] || exit 1
docker compose -f compose.dev.yaml up -d --build --no-deps airrace-web
docker compose -f compose.dev.yaml ps airrace-web
echo "Deployed source SHA $sha; previous SHA $previous. Public smoke is performed by CI next."
# No destructive automatic reset/rollback. Preserve server state on failure.
REMOTE
