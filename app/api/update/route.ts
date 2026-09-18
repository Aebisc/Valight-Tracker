import { NextResponse } from "next/server";
import packageJson from "@/package.json";
import fs from "fs";
import path from "path";
import os from "os";
import { spawn, exec } from "child_process";

export const dynamic = "force-dynamic";

const LATEST_JSON_URL = "https://github.com/Aebisc/Valight-Tracker/releases/latest/download/latest.json";

function isNewer(latest: string, current: string): boolean {
  const parse = (v: string) => v.replace(/^v/i, "").split(".").map(n => parseInt(n, 10) || 0);
  const l = parse(latest);
  const c = parse(current);
  for (let i = 0; i < Math.max(l.length, c.length); i++) {
    const lPart = l[i] ?? 0;
    const cPart = c[i] ?? 0;
    if (lPart > cPart) return true;
    if (lPart < cPart) return false;
  }
  return false;
}

export async function GET() {
  const currentVersion = packageJson.version;
  try {
    const res = await fetch(LATEST_JSON_URL, {
      cache: "no-store",
      headers: { "Accept": "application/json" },
      signal: AbortSignal.timeout(8000),
    });

    if (!res.ok) {
      return NextResponse.json({ available: false, currentVersion, error: `HTTP ${res.status}` });
    }

    const data = await res.json();
    const latestVersion = data.version;
    const notes = data.notes || "Bug fixes and improvements.";
    const url = data.platforms?.["windows-x86_64"]?.url || "";

    if (latestVersion && isNewer(latestVersion, currentVersion)) {
      return NextResponse.json({
        available: true,
        currentVersion,
        version: latestVersion,
        notes,
        pubDate: data.pub_date,
        url,
      });
    }

    return NextResponse.json({
      available: false,
      currentVersion,
      version: latestVersion || currentVersion,
    });
  } catch (err: any) {
    return NextResponse.json({
      available: false,
      currentVersion,
      error: err.message || "Failed to check for updates",
    });
  }
}

export async function POST(req: Request) {
  try {
    const body = await req.json();
    const downloadUrl = body.url;

    if (!downloadUrl || typeof downloadUrl !== "string" || !downloadUrl.startsWith("https://github.com/Aebisc/Valight-Tracker/releases/download/")) {
      return NextResponse.json({ error: "Invalid download URL" }, { status: 400 });
    }

    const res = await fetch(downloadUrl, {
      signal: AbortSignal.timeout(60000),
    });

    if (!res.ok || !res.body) {
      return NextResponse.json({ error: `Failed to download: HTTP ${res.status}` }, { status: 500 });
    }

    const contentLength = parseInt(res.headers.get("content-length") || "0", 10);
    const tempExePath = path.join(os.tmpdir(), "VaLight-Tracker-Update-Setup.exe");
    const fileStream = fs.createWriteStream(tempExePath);

    const encoder = new TextEncoder();
    const stream = new ReadableStream({
      async start(controller) {
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ event: "Started", total: contentLength })}\n\n`)
        );

        let downloaded = 0;
        const reader = res.body!.getReader();

        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            if (value) {
              fileStream.write(Buffer.from(value));
              downloaded += value.byteLength;
              controller.enqueue(
                encoder.encode(`data: ${JSON.stringify({ event: "Progress", downloaded, total: contentLength })}\n\n`)
              );
            }
          }

          await new Promise<void>((resolve, reject) => {
            fileStream.end((err?: Error | null) => {
              if (err) reject(err);
              else resolve();
            });
          });

          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify({ event: "Finished" })}\n\n`)
          );
          controller.close();

          // Wait 1.5s for client to receive "Finished" event and render "Installing...",
          // then spawn the installer and exit cleanly so files can be overwritten.
          setTimeout(() => {
            try {
              const installer = spawn(tempExePath, [], {
                detached: true,
                stdio: "ignore",
              });
              installer.unref();

              setTimeout(() => {
                exec("taskkill /F /IM valorant-tracker.exe", () => {
                  process.exit(0);
                });
              }, 500);
            } catch (spawnErr) {
              console.error("Failed to spawn installer:", spawnErr);
            }
          }, 1500);

        } catch (streamErr: any) {
          fileStream.close();
          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify({ event: "Error", message: streamErr.message })}\n\n`)
          );
          controller.close();
        }
      },
    });

    return new Response(stream, {
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        "Connection": "keep-alive",
      },
    });
  } catch (err: any) {
    return NextResponse.json({ error: err.message || "Update failed" }, { status: 500 });
  }
}
