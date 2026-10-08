/*
 * cybOS Runtime Layer 0.8
 * Local-first cooperative cell runtime for the browser prototype.
 *
 * Every cell exposes:
 *   - inputs / outputs
 *   - status
 *   - heartbeat
 *   - bounded execution budget
 *
 * JavaScript cannot hard-kill a synchronous function, so the budget is
 * cooperative: execution is measured, overruns are recorded, and the
 * runtime marks the cell degraded instead of pretending it was preempted.
 */

(() => {
  const startedAt = performance.now();

  class CybCell {
    constructor(id, spec = {}) {
      this.id = id;
      this.inputs = spec.inputs || [];
      this.outputs = spec.outputs || [];
      this.budgetMs = spec.budgetMs ?? 8;
      this.status = "BOOT";
      this.lastHeartbeat = 0;
      this.lastRun = 0;
      this.lastDuration = 0;
      this.runs = 0;
      this.overruns = 0;
      this.errors = 0;
      this.lastError = "";
      this.boundObject = null;
      this.boundMethod = null;
    }

    heartbeat(now = performance.now()) {
      this.lastHeartbeat = now;
      if (this.status !== "ERROR") this.status = "ONLINE";
    }

    execute(fn) {
      const started = performance.now();
      this.status = "RUNNING";

      try {
        const result = fn();
        const duration = performance.now() - started;
        this.lastDuration = duration;
        this.lastRun = performance.now();
        this.runs += 1;
        this.heartbeat(this.lastRun);

        if (duration > this.budgetMs) {
          this.overruns += 1;
          this.status = "DEGRADED";
        }

        return result;
      } catch (error) {
        this.errors += 1;
        this.status = "ERROR";
        this.lastError = String(error?.message || error);
        return undefined;
      }
    }

    snapshot(now = performance.now()) {
      const age = this.lastHeartbeat ? now - this.lastHeartbeat : Infinity;
      const heartbeat = age <= 1500 ? "OK" : age <= 4000 ? "LATE" : "STALE";

      if (this.status === "ONLINE" && heartbeat !== "OK") {
        this.status = heartbeat === "LATE" ? "DEGRADED" : "OFFLINE";
      }

      return {
        id: this.id,
        status: this.status,
        heartbeat,
        ageMs: Number.isFinite(age) ? Math.round(age) : null,
        budgetMs: this.budgetMs,
        lastDurationMs: Number(this.lastDuration.toFixed(2)),
        runs: this.runs,
        overruns: this.overruns,
        errors: this.errors,
        inputs: [...this.inputs],
        outputs: [...this.outputs],
        lastError: this.lastError
      };
    }
  }

  class CybRuntime {
    constructor() {
      this.version = "0.8.0";
      this.node = "cybOS-demo";
      this.startedAt = startedAt;
      this.cells = new Map();
      this.events = [];
      this.visible = false;
      this.maxEvents = 80;
    }

    register(id, spec = {}) {
      if (!this.cells.has(id)) {
        this.cells.set(id, new CybCell(id, spec));
      }
      return this.cells.get(id);
    }

    attach(id, object, method = "update") {
      const cell = this.cells.get(id);
      if (!cell || !object || typeof object[method] !== "function") return;

      if (object[`__cybWrapped_${method}`]) return;
      object[`__cybWrapped_${method}`] = true;

      const original = object[method];
      const runtime = this;

      object[method] = function(...args) {
        return cell.execute(() => original.apply(this, args));
      };

      cell.boundObject = object;
      cell.boundMethod = method;

      this.event("CELL_ATTACHED", { cell: id, method });
    }

    run(id, fn) {
      const cell = this.cells.get(id);
      if (!cell) return fn();
      return cell.execute(fn);
    }

    heartbeat(id) {
      const cell = this.cells.get(id);
      if (cell) cell.heartbeat();
    }

    event(type, payload = {}) {
      this.events.push({
        time: new Date().toISOString(),
        type,
        payload
      });
      if (this.events.length > this.maxEvents) this.events.shift();
    }

    snapshots() {
      const now = performance.now();
      return [...this.cells.values()].map(cell => cell.snapshot(now));
    }

    healthyCount() {
      return this.snapshots().filter(c =>
        c.status === "ONLINE" && c.heartbeat === "OK"
      ).length;
    }

    tick() {
      this.attachIfReady();
    }

    attachIfReady() {
      const bindings = [
        ["RADAR", window.radar],
        ["CYBCORE", window.core],
        ["SIGNAL_GRID", window.grid],
        ["CYBCHAT", window.spectrum],
        ["CYBERGRAPH", window.glyphs],
        ["HOLOGRAM", window.holo3d]
      ];

      for (const [id, object] of bindings) {
        if (object) this.attach(id, object, "update");
      }

      if (window.terminal) this.heartbeat("TERMINAL");
      if (window.CTRL) this.heartbeat("SYSTEM");
    }

    togglePanel() {
      this.visible = !this.visible;
      const panel = document.getElementById("cyb-runtime-panel");
      if (panel) panel.classList.toggle("open", this.visible);
    }

    render() {
      const status = document.getElementById("cyb-runtime-status");
      const panel = document.getElementById("cyb-runtime-panel");
      if (!status || !panel) return;

      const snapshots = this.snapshots();
      const healthy = this.healthyCount();

      status.textContent = `RUNTIME ${healthy}/${snapshots.length || 0} ONLINE`;
      status.dataset.state =
        healthy === snapshots.length && snapshots.length > 0 ? "ok" :
        healthy > 0 ? "warn" : "error";

      panel.innerHTML = `
        <div class="cyb-runtime-head">
          <span>CYB RUNTIME // ${this.version}</span>
          <button id="cyb-runtime-close" aria-label="Close runtime panel">×</button>
        </div>
        <div class="cyb-runtime-grid">
          ${snapshots.map(c => `
            <div class="cyb-cell" data-state="${c.status}">
              <div class="cyb-cell-top">
                <strong>${c.id}</strong>
                <span>${c.status}</span>
              </div>
              <div class="cyb-cell-meta">
                HB:${c.heartbeat}
                · ${c.lastDurationMs}ms/${c.budgetMs}ms
                · RUN:${c.runs}
                · OVR:${c.overruns}
              </div>
              <div class="cyb-cell-io">
                IN: ${c.inputs.join(", ") || "—"}<br>
                OUT: ${c.outputs.join(", ") || "—"}
              </div>
            </div>
          `).join("")}
        </div>
        <div class="cyb-runtime-foot">
          local-first · cooperative bounded runtime · no false connectivity
        </div>
      `;

      const close = document.getElementById("cyb-runtime-close");
      if (close) close.onclick = () => this.togglePanel();
    }
  }

  window.CybRuntime = CybRuntime;
  window.CYB_RUNTIME = new CybRuntime();

  const rt = window.CYB_RUNTIME;

  rt.register("SYSTEM", {
    inputs: ["clock", "keyboard", "pointer"],
    outputs: ["global state", "events"],
    budgetMs: 4
  });

  rt.register("RADAR", {
    inputs: ["system state", "scan command"],
    outputs: ["contacts", "scan history"],
    budgetMs: 4
  });

  rt.register("CYBCORE", {
    inputs: ["throttle", "warp", "power"],
    outputs: ["energy", "resonance", "status"],
    budgetMs: 5
  });

  rt.register("SIGNAL_GRID", {
    inputs: ["core state", "throttle"],
    outputs: ["packets", "activity"],
    budgetMs: 5
  });

  rt.register("CYBCHAT", {
    inputs: ["messages", "local events"],
    outputs: ["conversation state"],
    budgetMs: 5
  });

  rt.register("CYBERGRAPH", {
    inputs: ["entities", "relations", "events"],
    outputs: ["nodes", "connections", "signals"],
    budgetMs: 6
  });

  rt.register("HOLOGRAM", {
    inputs: ["system state", "target"],
    outputs: ["projection"],
    budgetMs: 4
  });

  rt.register("TERMINAL", {
    inputs: ["commands"],
    outputs: ["events", "state changes"],
    budgetMs: 8
  });

  window.addEventListener("keydown", event => {
    if (event.key.toLowerCase() === "r" && !event.metaKey && !event.ctrlKey) {
      rt.togglePanel();
      rt.event("RUNTIME_PANEL", { visible: rt.visible });
    }
  });

  setInterval(() => rt.tick(), 250);
  setInterval(() => rt.render(), 500);
})();
