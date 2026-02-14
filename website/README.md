# Openportio Docs Site

This folder contains the VitePress documentation portal.

## Run Locally

```bash
cd website
npm install
npm run docs:dev
```

## Build

```bash
cd website
npm run docs:build
```

## Validate Links

```bash
cd website
npm run docs:check-links
```

## GitHub Pages Deploy

CI builds and deploys this site via GitHub Actions on pushes to `develop`/`main`.
Production env defaults:

```bash
VITEPRESS_BASE=/Openportio/
VITEPRESS_SITE_ORIGIN=https://jaeyoung0509.github.io
```
