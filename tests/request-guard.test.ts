import { test } from "node:test";
import assert from "node:assert";

// Model the state machine and request synchronization behavior of useMatchPolling
class RequestSyncController {
  requestId = 0;
  isFetching = false;
  state = {
    gameState: "OFFLINE",
    matchId: null as string | null,
    error: "",
    rescheduledState: null as string | null,
  };

  async fetch(manual: boolean, fetchPromise: Promise<{ gameState: string; matchId: string }>): Promise<void> {
    if (this.isFetching && !manual) return;
    this.isFetching = true;
    const reqId = ++this.requestId;

    try {
      const data = await fetchPromise;
      if (reqId !== this.requestId) return; // Dropped if stale

      this.state.gameState = data.gameState;
      this.state.matchId = data.matchId;
      this.state.error = "";
      this.state.rescheduledState = data.gameState;
    } catch (e: any) {
      if (reqId !== this.requestId) return; // Dropped if stale

      this.state.error = e.message || "Failed";
    } finally {
      if (reqId === this.requestId) {
        this.isFetching = false;
      }
    }
  }

  unmount() {
    this.requestId++;
  }
}

function defer<T>() {
  let resolve!: (val: T) => void;
  let reject!: (err: any) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

test("BUG-01: Out-of-order responses - older slower request A is dropped when newer request B completes first", async () => {
  const ctrl = new RequestSyncController();

  const reqA = defer<{ gameState: string; matchId: string }>();
  const reqB = defer<{ gameState: string; matchId: string }>();

  // Dispatch A (t=0)
  const pA = ctrl.fetch(false, reqA.promise);
  assert.strictEqual(ctrl.isFetching, true);
  assert.strictEqual(ctrl.requestId, 1);

  // Dispatch B (manual refresh at t=100)
  const pB = ctrl.fetch(true, reqB.promise);
  assert.strictEqual(ctrl.requestId, 2);

  // Resolve B first (t=200)
  reqB.resolve({ gameState: "INGAME", matchId: "match-B" });
  await pB;

  assert.strictEqual(ctrl.state.gameState, "INGAME");
  assert.strictEqual(ctrl.state.matchId, "match-B");
  assert.strictEqual(ctrl.state.rescheduledState, "INGAME");
  assert.strictEqual(ctrl.isFetching, false);

  // Now resolve A later (t=500)
  reqA.resolve({ gameState: "MENUS", matchId: "match-A" });
  await pA;

  // State must NOT be overwritten by A!
  assert.strictEqual(ctrl.state.gameState, "INGAME", "State should remain B's state");
  assert.strictEqual(ctrl.state.matchId, "match-B", "Match should remain B's match");
  assert.strictEqual(ctrl.state.rescheduledState, "INGAME");
});

test("BUG-01: Unmount while request is pending drops late responses", async () => {
  const ctrl = new RequestSyncController();
  const req = defer<{ gameState: string; matchId: string }>();

  const p = ctrl.fetch(false, req.promise);
  assert.strictEqual(ctrl.isFetching, true);

  // Simulate unmount
  ctrl.unmount();

  // Resolve the deferred request
  req.resolve({ gameState: "INGAME", matchId: "match-late" });
  await p;

  // State should not have updated
  assert.strictEqual(ctrl.state.gameState, "OFFLINE");
  assert.strictEqual(ctrl.state.matchId, null);
});

test("BUG-01: Newest request fails while older succeeds - newer failure takes precedence", async () => {
  const ctrl = new RequestSyncController();

  const reqA = defer<{ gameState: string; matchId: string }>();
  const reqB = defer<{ gameState: string; matchId: string }>();

  // A starts, then B starts
  const pA = ctrl.fetch(false, reqA.promise);
  const pB = ctrl.fetch(true, reqB.promise);

  // B fails
  reqB.reject(new Error("Network timeout on newest request"));
  await pB;

  assert.strictEqual(ctrl.state.error, "Network timeout on newest request");

  // A succeeds afterwards
  reqA.resolve({ gameState: "INGAME", matchId: "match-A" });
  await pA;

  // Stale success must not overwrite the error from newer request
  assert.strictEqual(ctrl.state.error, "Network timeout on newest request");
  assert.strictEqual(ctrl.state.gameState, "OFFLINE");
  assert.strictEqual(ctrl.state.matchId, null);
});
