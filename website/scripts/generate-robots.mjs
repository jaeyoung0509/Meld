import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

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

const base = normalizeBase(process.env.VITEPRESS_BASE || '/');
const siteOrigin = (process.env.VITEPRESS_SITE_ORIGIN || 'https://jaeyoung0509.github.io').replace(
  /\/$/,
  ''
);
const basePrefix = base === '/' ? '' : base;
const sitemapUrl = `${siteOrigin}${basePrefix}sitemap.xml`;
const robotsPath = path.resolve(__dirname, '..', 'docs', 'public', 'robots.txt');

mkdirSync(path.dirname(robotsPath), { recursive: true });
writeFileSync(
  robotsPath,
  `User-agent: *\nAllow: /\nSitemap: ${sitemapUrl}\n`,
  'utf8'
);

console.log(`[docs:prepare] wrote ${path.relative(process.cwd(), robotsPath)} with sitemap ${sitemapUrl}`);
