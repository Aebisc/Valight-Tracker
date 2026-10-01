/**
 * redact-fixtures.ts
 *
 * Reads raw Riot API responses from src-tauri/fixtures/raw/{scenario}/
 * and writes redacted copies to src-tauri/fixtures/redacted/{scenario}/
 * with PUUIDs, names, and tokens replaced with deterministic fake values.
 *
 * Usage:
 *   npx tsx scripts/redact-fixtures.ts --scenario pregame_comp
 *
 * Safe to commit: redacted/ has no real credentials or player identities.
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";

const SCENARIO = process.argv.find((a) => a.startsWith("--scenario="))?.split("=")[1]
  ?? process.argv[process.argv.indexOf("--scenario") + 1];

if (!SCENARIO) {
  console.error("Usage: npx tsx scripts/redact-fixtures.ts --scenario <name>");
  process.exit(1);
}

const RAW_DIR = path.join(process.cwd(), "src-tauri", "fixtures", "raw", SCENARIO);
const OUT_DIR = path.join(process.cwd(), "src-tauri", "fixtures", "redacted", SCENARIO);

// Deterministic PUUID map: real → fake
const puuidMap = new Map<string, string>();
let puuidSeq = 0;

function fakePuuid(real: string): string {
  if (!puuidMap.has(real)) {
    puuidSeq++;
    puuidMap.set(real, `aaaaaaaa-${String(puuidSeq).padStart(4, "0")}-0000-0000-000000000000`);
  }
  return puuidMap.get(real)!;
}

// Name map: GameName / TagLine
const nameMap = new Map<string, string>();
let nameSeq = 0;
function fakeName(real: string): string {
  if (!nameMap.has(real)) {
    nameSeq++;
    nameMap.set(real, `Player${nameSeq}`);
  }
  return nameMap.get(real)!;
}

let tagSeq = 0;
const tagMap = new Map<string, string>();
function fakeTag(real: string): string {
  if (!tagMap.has(real)) {
    tagSeq++;
    tagMap.set(real, `TAG${tagSeq}`);
  }
  return tagMap.get(real)!;
}

const PUUID_RE = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi;

function redactString(s: string): string {
  return s.replace(PUUID_RE, (match) => fakePuuid(match.toLowerCase()));
}

function redactValue(v: unknown, key?: string): unknown {
  if (v === null || v === undefined) return v;

  if (typeof v === "string") {
    // Token fields
    if (key === "accessToken" || key === "token") return "fake-access-token";
    if (key === "entitlementsToken") return "fake-entitlements-token";
    // Name fields
    if (key === "GameName" || key === "DisplayName") return fakeName(v);
    if (key === "TagLine") return fakeTag(v);
    // Generic PUUID replacement in strings
    return redactString(v);
  }

  if (Array.isArray(v)) {
    return v.map((item) => redactValue(item));
  }

  if (typeof v === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(v as Record<string, unknown>)) {
      out[k] = redactValue(val, k);
    }
    return out;
  }

  return v;
}

async function redactFile(filename: string) {
  const inPath = path.join(RAW_DIR, filename);
  const outPath = path.join(OUT_DIR, filename);

  const raw = await readFile(inPath, "utf-8");

  // lockfile.txt is plain text — redact password field (index 3)
  if (filename === "lockfile.txt") {
    const parts = raw.trim().split(":");
    if (parts.length >= 5) {
      parts[3] = "fakepassword";
      await writeFile(outPath, parts.join(":"), "utf-8");
    } else {
      await writeFile(outPath, raw, "utf-8");
    }
    console.log(`  ✓  ${filename}`);
    return;
  }

  // shootergame_log.txt — redact any PUUIDs and tokens embedded in log lines
  if (filename === "shootergame_log.txt") {
    await writeFile(outPath, redactString(raw), "utf-8");
    console.log(`  ✓  ${filename}`);
    return;
  }

  // All other files are JSON
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    console.warn(`  ! ${filename} is not valid JSON, copying as-is`);
    await writeFile(outPath, raw, "utf-8");
    return;
  }

  const redacted = redactValue(parsed);
  await writeFile(outPath, JSON.stringify(redacted, null, 2), "utf-8");
  console.log(`  ✓  ${filename}`);
}

async function main() {
  await mkdir(OUT_DIR, { recursive: true });
  console.log(`\nRedacting scenario: ${SCENARIO}`);
  console.log(`Input:  ${RAW_DIR}`);
  console.log(`Output: ${OUT_DIR}\n`);

  let files: string[];
  try {
    files = await readdir(RAW_DIR);
  } catch {
    console.error(`Raw dir not found: ${RAW_DIR}`);
    console.error("Run capture-fixtures.ts first.");
    process.exit(1);
  }

  for (const f of files.sort()) {
    await redactFile(f);
  }

  // Write the PUUID mapping so the generate-expected script can use it
  const mapObj = Object.fromEntries(puuidMap);
  await writeFile(path.join(OUT_DIR, "_puuid_map.json"), JSON.stringify(mapObj, null, 2), "utf-8");
  console.log(`  ✓  _puuid_map.json (${puuidMap.size} PUUIDs mapped)`);

  console.log(`\n✅ Redaction complete for scenario: ${SCENARIO}`);
  console.log("   Next: run  npx tsx scripts/generate-expected.ts --scenario", SCENARIO);
}

main().catch((err) => {
  console.error("Redaction failed:", err);
  process.exit(1);
});
