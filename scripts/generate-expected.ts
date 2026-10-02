import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { GET } from "./legacy-route";
import { clearApiConfigCache } from "../lib/valorant-api";

async function loadJson(filePath: string): Promise<any> {
  try {
    const content = await readFile(filePath, "utf-8");
    return JSON.parse(content);
  } catch {
    return null;
  }
}

async function setupScenarioEnvironment(scenarioDir: string): Promise<() => void> {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "valight-oracle-"));
  const lockfileDstDir = path.join(tempDir, "Riot Games", "Riot Client", "Config");
  const logDstDir = path.join(tempDir, "VALORANT", "Saved", "Logs");
  fs.mkdirSync(lockfileDstDir, { recursive: true });
  fs.mkdirSync(logDstDir, { recursive: true });

  const lockfileSrc = path.join(scenarioDir, "lockfile.txt");
  if (fs.existsSync(lockfileSrc)) {
    fs.copyFileSync(lockfileSrc, path.join(lockfileDstDir, "lockfile"));
  }

  const logSrc = path.join(scenarioDir, "shootergame_log.txt");
  if (fs.existsSync(logSrc)) {
    fs.copyFileSync(logSrc, path.join(logDstDir, "ShooterGame.log"));
  }

  const prevLocalAppData = process.env.LOCALAPPDATA;
  process.env.LOCALAPPDATA = tempDir;

  return () => {
    if (prevLocalAppData !== undefined) {
      process.env.LOCALAPPDATA = prevLocalAppData;
    } else {
      delete process.env.LOCALAPPDATA;
    }
    fs.rmSync(tempDir, { recursive: true, force: true });
  };
}

function notFoundResponse(): Response {
  return new Response(JSON.stringify({ httpStatus: 404, message: "Not found" }), {
    status: 404,
    headers: { "Content-Type": "application/json" },
  });
}

function installFetchStub(scenarioDir: string): () => void {
  const originalFetch = globalThis.fetch;

  globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const urlStr = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;

    // 1. entitlements
    if (urlStr.includes("/entitlements/v1/token")) {
      const data = await loadJson(path.join(scenarioDir, "entitlements.json"));
      if (!data) return notFoundResponse();
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 2. presences
    if (urlStr.includes("/chat/v4/presences")) {
      const data = await loadJson(path.join(scenarioDir, "presences.json"));
      if (!data) return new Response("{}", { status: 200, headers: { "Content-Type": "application/json" } });
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 3. core-game player
    if (urlStr.includes("/core-game/v1/players/")) {
      const data = await loadJson(path.join(scenarioDir, "coregame_player.json"));
      if (!data || data.httpStatus === 404 || data.MatchID === undefined) {
        return notFoundResponse();
      }
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 4. pregame player
    if (urlStr.includes("/pregame/v1/players/")) {
      const data = await loadJson(path.join(scenarioDir, "pregame_player.json"));
      if (!data || data.httpStatus === 404 || data.MatchID === undefined) {
        return notFoundResponse();
      }
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 5. core-game match
    if (urlStr.includes("/core-game/v1/matches/")) {
      const data = await loadJson(path.join(scenarioDir, "coregame_match.json"));
      if (!data || data.httpStatus === 404) {
        return notFoundResponse();
      }
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 6. pregame match
    if (urlStr.includes("/pregame/v1/matches/")) {
      const data = await loadJson(path.join(scenarioDir, "pregame_match.json"));
      if (!data || data.httpStatus === 404) {
        return notFoundResponse();
      }
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 7. competitive updates: /mmr/v1/players/{id}/competitiveupdates
    const compMatch = urlStr.match(/\/mmr\/v1\/players\/([^\/?]+)\/competitiveupdates/);
    if (compMatch) {
      const puuid = compMatch[1];
      const data = await loadJson(path.join(scenarioDir, `comp_${puuid}.json`));
      if (!data || data.httpStatus === 404) {
        return notFoundResponse();
      }
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 8. mmr: /mmr/v1/players/{id}
    const mmrMatch = urlStr.match(/\/mmr\/v1\/players\/([^\/?]+)/);
    if (mmrMatch) {
      const puuid = mmrMatch[1];
      const data = await loadJson(path.join(scenarioDir, `mmr_${puuid}.json`));
      if (!data || data.httpStatus === 404) {
        return notFoundResponse();
      }
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 9. match details: /match-details/v1/matches/{id}
    const detailMatch = urlStr.match(/\/match-details\/v1\/matches\/([^\/?]+)/);
    if (detailMatch) {
      const matchId = detailMatch[1];
      const data = await loadJson(path.join(scenarioDir, `match_${matchId}.json`));
      if (!data || data.httpStatus === 404) {
        return notFoundResponse();
      }
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    // 10. names: /name-service/v2/players
    if (urlStr.includes("/name-service/v2/players")) {
      const data = await loadJson(path.join(scenarioDir, "names.json"));
      if (!data) return new Response("[]", { status: 200, headers: { "Content-Type": "application/json" } });
      return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
    }

    return notFoundResponse();
  }) as any;

  return () => {
    globalThis.fetch = originalFetch;
  };
}

async function processScenario(scenario: string) {
  const redactedDir = path.join(process.cwd(), "src-tauri", "fixtures", "redacted", scenario);
  const outDir = path.join(process.cwd(), "src-tauri", "fixtures", "expected");
  await mkdir(outDir, { recursive: true });

  console.log(`\nGenerating oracle for: ${scenario}`);

  const cleanupEnv = await setupScenarioEnvironment(redactedDir);
  const cleanupFetch = installFetchStub(redactedDir);
  clearApiConfigCache();

  try {
    const request = new Request("http://localhost/api/match?force=1");
    const response = await GET(request);
    const json = await response.json();

    const outFile = path.join(outDir, `${scenario}.json`);
    await writeFile(outFile, JSON.stringify(json, null, 2), "utf-8");
    console.log(`  ✓ Generated oracle for ${scenario} -> ${outFile}`);
  } finally {
    cleanupFetch();
    cleanupEnv();
  }
}

async function main() {
  const all = process.argv.includes("--all");
  const scenario = process.argv.find((a) => a.startsWith("--scenario="))?.split("=")[1]
    ?? process.argv[process.argv.indexOf("--scenario") + 1];

  if (all) {
    const redactedBase = path.join(process.cwd(), "src-tauri", "fixtures", "redacted");
    const scenarios = await readdir(redactedBase).catch(() => [] as string[]);
    for (const s of scenarios) {
      await processScenario(s);
    }
  } else if (scenario) {
    await processScenario(scenario);
  } else {
    console.error("Usage: npx tsx scripts/generate-expected.ts --scenario <name> | --all");
    process.exit(1);
  }
}

main().catch((err) => {
  console.error("Generation failed:", err);
  process.exit(1);
});
