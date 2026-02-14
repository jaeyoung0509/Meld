# Docs Site Ops Runbook

This runbook covers incident response for the VitePress docs pipeline in `.github/workflows/docs-site.yml`.

## Pipeline Gates

`Docs Site` workflow enforces:
- docs markdown internal link validation (`npm run docs:check-links`)
- static site build (`npm run docs:build`)
- generated output checks:
  - `website/docs/.vitepress/dist/sitemap.xml`
  - `website/docs/.vitepress/dist/robots.txt`
  - expected sitemap URL in `robots.txt`
- post-deploy smoke check via `./scripts/check_docs_site_smoke.sh`

## Local Reproduction

From repository root:

```bash
cd website
npm ci
npm run docs:check-links
VITEPRESS_BASE=/Openportio/ \
VITEPRESS_SITE_ORIGIN=https://jaeyoung0509.github.io \
npm run docs:build
```

Validate generated artifacts:

```bash
test -f website/docs/.vitepress/dist/sitemap.xml
test -f website/docs/.vitepress/dist/robots.txt
cat website/docs/.vitepress/dist/robots.txt
```

Optional local smoke check:

```bash
cd website
VITEPRESS_BASE=/Openportio/ npm run docs:preview -- --host 127.0.0.1 --port 4173
```

In another shell:

```bash
./scripts/check_docs_site_smoke.sh http://127.0.0.1:4173/Openportio/ Openportio
```

## Failure Triage

- `docs:check-links` failed
  - fix missing/renamed docs links in `website/docs/**/*.md`
  - rerun `npm run docs:check-links`
- missing `sitemap.xml` / `robots.txt`
  - rerun build with expected env (`VITEPRESS_BASE`, `VITEPRESS_SITE_ORIGIN`)
  - confirm `website/scripts/generate-robots.mjs` runs in build
- smoke check failed after deploy
  - inspect workflow step logs for headers/body snippet
  - manually `curl -iL <page_url>` to confirm status and marker text
  - if GitHub Pages propagation delay is suspected, rerun failed workflow

## Rollback / Retry

1. Identify last known-good docs commit on `develop`/`main`.
2. Revert problematic docs-site commit(s) in a PR targeting `develop`.
3. Merge revert PR and confirm `Docs Site` workflow is green.
4. If only deployment flaked (content is correct), use Actions "Re-run failed jobs".
5. Verify production URL after recovery:
   - `https://jaeyoung0509.github.io/Openportio/`
