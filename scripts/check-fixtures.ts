import fs from "node:fs";
import path from "node:path";

const FIXTURES_DIR = path.resolve(__dirname, "../src-tauri/fixtures");
const REDACTED_DIR = path.join(FIXTURES_DIR, "redacted");
const EXPECTED_DIR = path.join(FIXTURES_DIR, "expected");

let hasErrors = false;

function error(msg: string) {
  console.error(`❌ [check-fixtures] ${msg}`);
  hasErrors = true;
}

function success(msg: string) {
  console.log(`✓ [check-fixtures] ${msg}`);
}

// 1. Check scenarios and expected files
if (!fs.existsSync(REDACTED_DIR)) {
  error(`Redacted directory not found: ${REDACTED_DIR}`);
  process.exit(1);
}

const scenarios = fs
  .readdirSync(REDACTED_DIR, { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name);

for (const scenario of scenarios) {
  const expectedPath = path.join(EXPECTED_DIR, `${scenario}.json`);
  if (!fs.existsSync(expectedPath)) {
    error(`Missing expected oracle file for scenario '${scenario}': ${expectedPath}`);
  } else {
    success(`Scenario '${scenario}' has expected file: ${expectedPath}`);
  }
}

// 2. Scan all files in redacted and expected for JWT or real PUUID leakage
const jwtRegex = /\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/g;

function scanDir(dir: string) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      scanDir(fullPath);
    } else if (entry.isFile() && (entry.name.endsWith(".json") || entry.name.endsWith(".txt"))) {
      const content = fs.readFileSync(fullPath, "utf-8");

      // Check JWT
      const jwtMatches = content.match(jwtRegex);
      if (jwtMatches) {
        error(`Found unredacted signed JWT in ${fullPath}: ${jwtMatches[0].slice(0, 20)}...`);
      }

      // Check for presence of obvious unredacted riot IDs or patterns
      if (content.includes("Bouguelli") || content.includes("Asc#")) {
        error(`Found known unredacted player name in ${fullPath}`);
      }
    }
  }
}

scanDir(REDACTED_DIR);
scanDir(EXPECTED_DIR);

if (hasErrors) {
  console.error("\n❌ Fixture hygiene checks failed!");
  process.exit(1);
} else {
  console.log("\n✓ All fixture hygiene checks passed cleanly!");
  process.exit(0);
}
