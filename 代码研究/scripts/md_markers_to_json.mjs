#!/usr/bin/env bun
/**
 * Offline converter: Markdown 标识物 table → asr_markers.json
 *
 * Does NOT hardcode domain terms. Reads any markdown table of the form:
 *
 *   | ASR误识 / 误识 | 正确形式 / 正确 | 类别 |
 *   | --- | --- | --- |
 *   | 错词A / 错词B | 正确词 | term |
 *
 * Usage:
 *   bun 代码研究/scripts/md_markers_to_json.mjs input.md [-o out.json] [--verified] [--source label]
 *
 * Output is compatible with Handy `asr_markers.json` (version 1).
 */

import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

function usage() {
  console.error(`Usage: bun md_markers_to_json.mjs <input.md> [-o out.json] [--verified] [--source label]
  Default confidence: pending (import for review).
  --verified marks all rows as verified (use only for curated lists).`);
  process.exit(1);
}

const args = process.argv.slice(2);
if (args.length === 0 || args.includes("-h") || args.includes("--help")) {
  usage();
}

let input = null;
let output = null;
let verified = false;
let source = "offline_md_import";

for (let i = 0; i < args.length; i++) {
  const a = args[i];
  if (a === "-o" || a === "--output") {
    output = args[++i];
  } else if (a === "--verified") {
    verified = true;
  } else if (a === "--source") {
    source = args[++i] ?? source;
  } else if (!a.startsWith("-")) {
    input = a;
  }
}

if (!input) usage();

const raw = readFileSync(resolve(input), "utf8");
const confidence = verified ? "verified" : "pending";
const markers = [];

for (const line of raw.split(/\r?\n/)) {
  const trimmed = line.trim();
  if (!trimmed.startsWith("|")) continue;
  const cells = trimmed
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());
  if (cells.length < 2) continue;
  const [c0, c1, c2] = cells;
  if (!c0 || !c1) continue;
  if (
    c0.includes("---") ||
    c1.includes("---") ||
    /ASR|误识|error/i.test(c0) ||
    /正确|correct/i.test(c1)
  ) {
    continue;
  }
  const asr_errors = c0
    .split(/[/、,，]/)
    .map((s) => s.trim())
    .filter(Boolean);
  if (asr_errors.length === 0) continue;
  markers.push({
    asr_errors,
    correct: c1,
    category: (c2 && c2.trim()) || "other",
    note: null,
    source,
    confidence,
  });
}

const today = new Date().toISOString().slice(0, 10);
const store = {
  version: 1,
  markers,
  changelog: [
    {
      date: today,
      summary: `Offline import of ${markers.length} marker(s) from ${input}`,
      source,
      count: markers.length,
    },
  ],
};

const json = JSON.stringify(store, null, 2) + "\n";
if (output) {
  writeFileSync(resolve(output), json, "utf8");
  console.error(`Wrote ${markers.length} marker(s) → ${output}`);
} else {
  process.stdout.write(json);
}
