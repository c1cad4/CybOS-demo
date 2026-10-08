/*
 * Runtime bridge for the legacy p5 sketch.
 * The visual prototype keeps its rendering cells in one legacy script,
 * so this bridge provides a frame-level heartbeat without rewriting the
 * visual layer. The native cybOS implementation can later bind the same
 * cell contract to real workers/tasks.
 */
(() => {
  const rt = window.CYB_RUNTIME;
  if (!rt) return;

  const originalDraw = window.draw;
  if (typeof originalDraw === "function" && !originalDraw.__cybRuntimeWrapped) {
    function boundedDraw() {
      const started = performance.now();
      let result;
      try {
        result = originalDraw.apply(this, arguments);
      } finally {
        const elapsed = performance.now() - started;
        rt.event("FRAME", {
          durationMs: Number(elapsed.toFixed(2)),
          budgetMs: 16.67
        });

        // The legacy renderer is a single synchronous frame. Until the
        // renderer is split into workers/tasks, each logical cell receives
        // the frame heartbeat from this boundary.
        for (const id of [
          "SYSTEM", "RADAR", "CYBCORE", "SIGNAL_GRID",
          "CYBCHAT", "CYBERGRAPH", "HOLOGRAM"
        ]) {
          rt.heartbeat(id);
        }
      }
      return result;
    }

    boundedDraw.__cybRuntimeWrapped = true;
    window.draw = boundedDraw;
  }

  rt.event("RUNTIME_READY", {
    mode: "legacy-frame-bridge",
    note: "cell execution is cooperative until native task boundaries are introduced"
  });
})();
