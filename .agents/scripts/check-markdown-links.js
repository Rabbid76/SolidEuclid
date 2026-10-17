#!/usr/bin/env node
/**
 * Checks the relative links of markdown files: the target file exists, a
 * heading anchor (#some-heading) exists in a markdown target, a line anchor
 * (#L42, #L42-L51) is within the target file, and the target is inside the
 * repository (the working directory) and outside .temp - a link only this
 * machine resolves is reported as LOCAL. Links starting with http and links in
 * code blocks or inline code are skipped. An optional link title
 * ([text](file.md "title")) does not skip the destination check.
 *
 *   node .agents/scripts/check-markdown-links.js <file.md> [<file.md> ...]
 *
 * Prints one line per broken link and ends with "bad <count>"; the exit code
 * is 1 when a link is broken.
 */

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, normalize, relative } from 'node:path';

const anchorsByFile = new Map();

function slug(heading) {
  return heading
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{M}\p{N}_\- ]/gu, '')
    .replaceAll(' ', '-');
}

function proseLines(file) {
  const lines = [];
  let openFence = null;
  for (const line of readFileSync(file, 'utf8').split('\n')) {
    const fence = line.match(/^\s*(`{3,}|~{3,})/)?.[1];
    if (fence && !openFence) openFence = fence;
    else if (fence && fence[0] === openFence[0] && fence.length >= openFence.length)
      openFence = null;
    else if (!openFence) lines.push(line);
  }
  return lines;
}

function anchors(file) {
  if (anchorsByFile.has(file)) return anchorsByFile.get(file);
  const found = new Set();
  const seen = new Map();
  for (const line of proseLines(file)) {
    const heading = line.match(/^#+\s+(.*)/);
    if (!heading) continue;
    const base = slug(heading[1]);
    const count = seen.get(base) ?? 0;
    seen.set(base, count + 1);
    found.add(count === 0 ? base : `${base}-${count}`);
  }
  anchorsByFile.set(file, found);
  return found;
}

function linkTargets(file) {
  return proseLines(file).flatMap((line) =>
    [...line.replace(/`[^`]*`/g, '').matchAll(/\]\(([^)\s]+)[^)]*\)/g)].map(([, link]) => link)
  );
}

function lineCount(file) {
  const text = readFileSync(file, 'utf8');
  return text.split('\n').length - (text.endsWith('\n') ? 1 : 0);
}

let bad = 0;
for (const file of process.argv.slice(2)) {
  for (const link of linkTargets(file)) {
    if (link.startsWith('http')) continue;
    const hashAt = link.indexOf('#');
    const path = hashAt < 0 ? link : link.slice(0, hashAt);
    const fragment = hashAt < 0 ? '' : link.slice(hashAt + 1);
    const target = path ? normalize(join(dirname(file), path)) : file;
    const fromRoot = relative(process.cwd(), target);
    if (fromRoot.startsWith('..') || fromRoot.startsWith('.temp')) {
      console.log(`${file}: LOCAL ${link}`);
      bad++;
    } else if (!existsSync(target)) {
      console.log(`${file}: MISSING ${link}`);
      bad++;
    } else if (/^L\d/.test(fragment)) {
      const lines = lineCount(target);
      if ([...fragment.matchAll(/L(\d+)/g)].some(([, line]) => Number(line) > lines)) {
        console.log(`${file}: LINE ${link}`);
        bad++;
      }
    } else if (fragment && target.endsWith('.md') && !anchors(target).has(fragment)) {
      console.log(`${file}: ANCHOR ${link}`);
      bad++;
    }
  }
}
console.log('bad', bad);
process.exitCode = bad ? 1 : 0;
