import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const docsRoot = path.resolve(__dirname, '..', 'docs');

function collectMarkdownFiles(dir) {
  const entries = readdirSync(dir, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === '.vitepress') {
        continue;
      }
      files.push(...collectMarkdownFiles(fullPath));
      continue;
    }
    if (entry.isFile() && entry.name.endsWith('.md')) {
      files.push(fullPath);
    }
  }

  return files;
}

function normalizeLinkTarget(rawTarget) {
  const trimmed = rawTarget.trim();
  if (trimmed.length === 0) {
    return null;
  }

  const unwrapped =
    trimmed.startsWith('<') && trimmed.endsWith('>')
      ? trimmed.slice(1, -1).trim()
      : trimmed.split(/\s+/)[0];

  if (
    unwrapped.startsWith('#') ||
    unwrapped.startsWith('http://') ||
    unwrapped.startsWith('https://') ||
    unwrapped.startsWith('mailto:') ||
    unwrapped.startsWith('tel:') ||
    unwrapped.startsWith('javascript:')
  ) {
    return null;
  }

  const withoutQuery = unwrapped.split('?')[0];
  const withoutFragment = withoutQuery.split('#')[0];
  return withoutFragment.length > 0 ? withoutFragment : null;
}

function pathCandidates(basePath) {
  if (path.extname(basePath)) {
    return [basePath];
  }
  return [basePath, `${basePath}.md`, path.join(basePath, 'index.md')];
}

function resolveCandidates(target, sourceFile) {
  if (target.startsWith('/')) {
    const relative = target.replace(/^\/+/, '');
    const fromDocsRoot = path.join(docsRoot, relative);
    const fromPublic = path.join(docsRoot, 'public', relative);
    return [...pathCandidates(fromDocsRoot), ...pathCandidates(fromPublic)];
  }

  const fromSource = path.resolve(path.dirname(sourceFile), target);
  return pathCandidates(fromSource);
}

function isExistingFile(candidate) {
  if (!existsSync(candidate)) {
    return false;
  }
  try {
    return statSync(candidate).isFile();
  } catch {
    return false;
  }
}

const markdownFiles = collectMarkdownFiles(docsRoot);
const linkPattern = /\[[^\]]+\]\(([^)]+)\)/g;
const failures = [];
let checkedLinkCount = 0;

function validateTarget(target, file, line) {
  const normalized = normalizeLinkTarget(target);
  if (!normalized) {
    return;
  }

  checkedLinkCount += 1;
  const candidates = resolveCandidates(normalized, file);
  const exists = candidates.some(isExistingFile);

  if (!exists) {
    failures.push({
      file,
      line,
      target: normalized,
      candidates
    });
  }
}

for (const file of markdownFiles) {
  const content = readFileSync(file, 'utf8');
  const lines = content.split('\n');

  lines.forEach((line, lineIndex) => {
    const matches = line.matchAll(linkPattern);
    for (const match of matches) {
      validateTarget(match[1], file, lineIndex + 1);
    }

    const frontmatterLinkMatch = line.match(/^\s*link:\s*["']?([^"' ]+)["']?\s*$/);
    if (frontmatterLinkMatch) {
      validateTarget(frontmatterLinkMatch[1], file, lineIndex + 1);
    }
  });
}

const configPath = path.join(docsRoot, '.vitepress', 'config.mts');
if (existsSync(configPath)) {
  const configLines = readFileSync(configPath, 'utf8').split('\n');
  configLines.forEach((line, lineIndex) => {
    const configLinkPattern = /link:\s*['"]([^'"]+)['"]/g;
    const matches = line.matchAll(configLinkPattern);
    for (const match of matches) {
      validateTarget(match[1], configPath, lineIndex + 1);
    }
  });
}

if (failures.length > 0) {
  console.error(`[FAIL] broken docs links detected: ${failures.length}`);
  for (const failure of failures) {
    console.error(
      `- ${path.relative(process.cwd(), failure.file)}:${failure.line} -> ${failure.target}`
    );
    console.error(`  checked: ${failure.candidates.map((item) => path.relative(process.cwd(), item)).join(', ')}`);
  }
  process.exit(1);
}

console.log(`[OK] docs links validated: ${checkedLinkCount} internal links checked across ${markdownFiles.length} markdown files.`);
