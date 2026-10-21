# Shortcuts

How to answer the questions that get asked about **which** over and over — prod URLs,
what's live, where the logs are. One command each, so nobody re-derives them from the
workflows and compose files every session.

**If you are an agent:**

- **Read this before any environment recon.** Question about the running system (prod,
  deploys, versions, logs, metrics, health)? The answer may already be below.
- **Write to it after any environment recon.** If answering took more than a couple of
  commands and the answer is durable, append an entry — in the same turn, before you
  report back.
- **Record the route, not the reading.** The endpoint, command, or mechanism — never
  "prod is on 1.4.212" or "deployed at 14:32". Those are stale within the hour.
- **Fix what's broken.** Ran a shortcut and it failed or the output changed? Correct the
  entry now; you're the one context that knows the new answer. Bump `_Verified:_` when a
  shortcut works.
- **No secrets.** Reference an env var or Komodo Variable; never paste the value.

Keep it under ~15 entries. Full convention: [shortcuts](https://standards.thmsn.dev/d/shortcuts).

---

## How do I install the cli, and does re-running it upgrade?

```sh
curl -fsSL https://apps.dev.thmsn.dev/install/dev.thmsn.which | sh
```

```powershell
irm https://apps.dev.thmsn.dev/install/dev.thmsn.which.ps1 | iex
```

The registry's installer picks the artifact for the machine, verifies its SHA-256 and
applies `which/packaging/install.json` (completions). Re-running it installs whatever is
live, so it is also the upgrade.

_Verified: never. Written from the registry's source; the first release has not run._

## What version of the cli is live, and what did a push turn into?

The registry lists versions per app at `https://apps.dev.thmsn.dev/api/v1/products/dev.thmsn.which/apps/cli`
(reads need a signed-in session; the API is browsable at `https://apps.dev.thmsn.dev/docs`).
Compare against local: `git rev-list --count HEAD` is the third component of the
[version](VERSIONING.md). A push to `main` touching the Rust tree runs `cli.release`; the
version is live once its `finalize` job is green, so "pushed" is not "installable".

_Verified: never. Written from the registry's source; the first release has not run._

## Is the gate green?

Woodpecker, this repo's pipelines: `rust.ci` (Linux), `rust.ci-matrix` (Linux, macOS,
Windows) and `cli.release`. Each generated file in [`.woodpecker/`](.woodpecker) is one
workflow. They run on pushes to `main` and by hand, never on pull requests.

_Verified: never. The repo has not been enabled in Woodpecker yet._

<!--
Format — one heading per question, phrased the way it actually gets asked:

## Is it up to date in prod?

```sh
curl -s https://<host>/api/build | jq -r '.version, .commit'
```

Compare against local: `git rev-list --count HEAD` (version is the commit count). Deploys
are Watchtower-polled every ~5m, so "pushed" ≠ "live".

_Verified: YYYY-MM-DD_

Worth answering early, if this repo has them:
  - Which host is prod? staging?
  - What version/commit is live right now?
  - What turns a push into a running container, and how long does it take?
  - Where are the logs? (the actual query, not "Grafana")
  - Where are the metrics/dashboards?
  - Where do images/artifacts get published?
  - How do I run this locally against real data?
-->
