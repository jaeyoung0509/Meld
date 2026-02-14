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

## GitHub Pages Deploy

CI builds and deploys this site via GitHub Actions on pushes to `develop`/`main`.
Production base path is set with:

```bash
VITEPRESS_BASE=/Openportio/
```
