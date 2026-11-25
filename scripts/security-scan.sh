#!/usr/bin/env bash
# Security scan with public, Docker-based scanners. Used by `make scan` and by the GitHub
# workflow (.github/workflows/security.yml), so local runs and CI apply the same checks.
#
#   scripts/security-scan.sh [image ...]          e.g. scripts/security-scan.sh nats-webui:dev
#
#   Semgrep CE   SAST over the source (registry ruleset p/default, no login, metrics off)
#   Trivy        vulnerabilities in every lockfile, secrets, Dockerfile misconfigurations,
#                and vulnerabilities in the given images
#
# It scans a clean copy of the files git would commit, so git-ignored secrets (.env, sandbox/,
# *.db) never reach a scanner. Images are passed as tarballs; the Docker socket is never mounted.
#
# Gate (SCAN_MODE=enforce exits 1, warn only reports):
#   - any Semgrep finding;
#   - Trivy: CRITICAL/HIGH vulnerabilities in lockfiles, any secret, CRITICAL/HIGH misconfigurations;
#   - Trivy: CRITICAL/HIGH vulnerabilities in images that have a fixed version.
#
# Reports: $SCAN_OUT (default .security-reports/<timestamp>/): JSON and SARIF per scanner,
# plus summary.md.
set -euo pipefail

# Pinned by version and digest: a moved tag can't change what runs.
SEMGREP_IMAGE="${SEMGREP_IMAGE:-semgrep/semgrep:1.186.0@sha256:6e1b04340e60e910fbb4ddbf3c857ab0918f064417ad5ac530014ccb3cf8c8f1}"
TRIVY_IMAGE="${TRIVY_IMAGE:-aquasec/trivy:0.76.2@sha256:8f6bb947dbc528902e11f2f5be52f7b519f6a39d5d148f41166f2040499768fb}"
SEMGREP_CONFIG="${SEMGREP_CONFIG:-p/default}"
mode="${SCAN_MODE:-warn}"

root="$(git rev-parse --show-toplevel)"
out="${SCAN_OUT:-$root/.security-reports/$(date +%Y%m%d-%H%M%S)}"
cache="${TRIVY_CACHE_DIR:-$root/.security-reports/.trivy-cache}"
mkdir -p "$out" "$cache"
out="$(cd "$out" && pwd)"
copy="$(mktemp -d)"
trap 'rm -rf "$copy"' EXIT
as_me=(--user "$(id -u):$(id -g)" -e HOME=/tmp)

echo "==> clean copy of committable files"
(cd "$root" && git ls-files -co --exclude-standard -z | while IFS= read -r -d '' f; do
  case "$f" in .env | sandbox/* | credential/*) continue ;; esac # never scan local secrets, even if tracked
  if [[ -f "$f" ]]; then cp --parents "$f" "$copy/"; fi          # skip files deleted in the working tree
done)
if git -C "$root" ls-files --error-unmatch .env >/dev/null 2>&1; then
  echo "    WARNING: .env is tracked by git; untrack it with: git rm --cached .env"
fi
[[ ! -e "$copy/sandbox" && ! -e "$copy/.env" ]] || { echo "refusing: secrets in scan copy" >&2; exit 1; }

trivy() {
  docker run --rm "${as_me[@]}" -v "$cache":/cache -v "$copy":/src:ro -v "$out":/out -w /src \
    "$TRIVY_IMAGE" --cache-dir /cache --quiet "$@"
}

echo "==> Semgrep ($SEMGREP_CONFIG)"
docker run --rm "${as_me[@]}" -v "$copy":/src:ro -v "$out":/out -w /src "$SEMGREP_IMAGE" \
  semgrep scan --config "$SEMGREP_CONFIG" --metrics=off --quiet \
  --json-output=/out/semgrep.json --sarif-output=/out/semgrep.sarif >/dev/null

echo "==> Trivy: lockfiles, secrets, Dockerfiles"
trivy fs --scanners vuln,secret,misconfig --format json -o /out/trivy-fs.json /src
trivy convert --format sarif -o /out/trivy-fs.sarif /out/trivy-fs.json

for img in "$@"; do
  name="${img//[\/:@]/_}"
  echo "==> Trivy: image $img"
  docker save "$img" -o "$out/$name.tar"
  trivy image --scanners vuln,secret --format json -o "/out/trivy-image-$name.json" --input "/out/$name.tar"
  trivy convert --format sarif -o "/out/trivy-image-$name.sarif" "/out/trivy-image-$name.json"
  rm -f "$out/$name.tar"
done

echo "==> summary"
python3 - "$out" "$SEMGREP_IMAGE" "$TRIVY_IMAGE" <<'PY'
import collections, json, pathlib, sys

out, semgrep_image, trivy_image = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
version = lambda image: image.split("@")[0].split(":")[-1]
rows, failures = [], []

sg = json.loads((out / "semgrep.json").read_text())
findings = sg.get("results", [])
rows.append(("Semgrep CE " + version(semgrep_image), "Source code", f"{len(findings)} findings", not findings))
for r in findings:
    failures.append(f"Semgrep {r['extra'].get('severity')} {r['check_id'].split('.')[-1]} {r['path']}:{r['start']['line']}")

def trivy_counts(path, fixed_only):
    d = json.loads(path.read_text())
    vulns, fixable, secrets, misconf = collections.Counter(), 0, 0, collections.Counter()
    for res in d.get("Results", []):
        for v in res.get("Vulnerabilities") or []:
            vulns[v["Severity"]] += 1
            if v["Severity"] in ("CRITICAL", "HIGH") and (v.get("FixedVersion") or not fixed_only):
                fixable += 1
                failures.append(f"Trivy {v['Severity']} {v['VulnerabilityID']} {v['PkgName']} {v.get('InstalledVersion')} -> {v.get('FixedVersion') or 'no fix'} ({res['Target']})")
        for s in res.get("Secrets") or []:
            secrets += 1
            failures.append(f"Trivy secret {s['RuleID']} in {res['Target']}")
        for m in res.get("Misconfigurations") or []:
            if m.get("Status") == "FAIL":
                misconf[m["Severity"]] += 1
                if m["Severity"] in ("CRITICAL", "HIGH"):
                    failures.append(f"Trivy misconfig {m['Severity']} {m['ID']} {m['Title']} ({res['Target']})")
    return vulns, fixable, secrets, misconf

tv = "Trivy " + version(trivy_image)
fmt = lambda c: f"{c['CRITICAL']} critical, {c['HIGH']} high, {c['MEDIUM']} medium, {c['LOW']} low"
v, bad, s, m = trivy_counts(out / "trivy-fs.json", fixed_only=False)
rows.append((tv, "Lockfiles (Cargo, npm, pip, Go)", fmt(v), bad == 0))
rows.append((tv, "Secrets", f"{s} found", s == 0))
rows.append((tv, "Dockerfiles", f"{m['CRITICAL']} critical, {m['HIGH']} high, {m['MEDIUM']} medium, {m['LOW']} low", m["CRITICAL"] + m["HIGH"] == 0))
for f in sorted(out.glob("trivy-image-*.json")):
    v, bad, s, _ = trivy_counts(f, fixed_only=True)
    rows.append((tv, "Image " + f.stem.removeprefix("trivy-image-"), fmt(v) + f"; {bad} fixable critical/high", bad == 0 and s == 0))

md = ["| Scanner | Target | Result | Gate |", "|---|---|---|---|"]
md += [f"| {a} | {b} | {c} | {'pass' if ok else '**FAIL**'} |" for a, b, c, ok in rows]
if failures:
    md += ["", "Blocking findings:", ""] + [f"- {x}" for x in failures]
text = "\n".join(md) + "\n"
(out / "summary.md").write_text(text)
(out / "blocking-count").write_text(str(sum(not ok for *_, ok in rows)))
print(text)
PY

echo "Reports: $out"
if [[ "$(cat "$out/blocking-count")" != 0 ]]; then
  if [[ "$mode" == enforce ]]; then
    echo "FAIL: blocking findings (see summary above)"
    exit 1
  fi
  echo "Blocking findings reported (SCAN_MODE=$mode, not failing)"
fi
echo "done (mode=$mode)"
