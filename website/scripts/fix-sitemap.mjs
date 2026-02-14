import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const sitemapPath = path.resolve(__dirname, '..', 'docs', '.vitepress', 'dist', 'sitemap.xml');

function normalizeBase(value) {
  let base = (value || '/').trim();
  if (base.length === 0) {
    base = '/';
  }
  if (!base.startsWith('/')) {
    base = `/${base}`;
  }
  if (!base.endsWith('/')) {
    base = `${base}/`;
  }
  return base;
}

function formatUrl(url) {
  if (url.pathname === '/') {
    return `${url.origin}/`;
  }
  const cleanPath = url.pathname.endsWith('/') ? url.pathname.slice(0, -1) : url.pathname;
  return `${url.origin}${cleanPath}${url.search}${url.hash}`;
}

if (!existsSync(sitemapPath)) {
  console.error(`[docs:build] sitemap not found: ${path.relative(process.cwd(), sitemapPath)}`);
  process.exit(1);
}

const base = normalizeBase(process.env.VITEPRESS_BASE || '/');
const siteOrigin = (process.env.VITEPRESS_SITE_ORIGIN || 'https://jaeyoung0509.github.io').replace(
  /\/$/,
  ''
);
const basePrefix = base === '/' ? '' : base.replace(/\/$/, '');

if (!basePrefix) {
  console.log('[docs:build] skip sitemap rewrite because base is "/"');
  process.exit(0);
}

const xml = readFileSync(sitemapPath, 'utf8');
let rewrites = 0;

const rewritten = xml.replace(/<loc>([^<]+)<\/loc>/g, (fullMatch, rawUrl) => {
  try {
    const url = new URL(rawUrl);
    if (url.origin !== siteOrigin) {
      return fullMatch;
    }

    if (url.pathname === basePrefix || url.pathname.startsWith(`${basePrefix}/`)) {
      return `<loc>${formatUrl(url)}</loc>`;
    }

    const suffix = url.pathname === '/' ? '' : url.pathname;
    url.pathname = `${basePrefix}${suffix}`;
    rewrites += 1;
    return `<loc>${formatUrl(url)}</loc>`;
  } catch {
    return fullMatch;
  }
});

writeFileSync(sitemapPath, rewritten, 'utf8');
console.log(`[docs:build] sitemap rewrite complete (${rewrites} entries updated)`);
