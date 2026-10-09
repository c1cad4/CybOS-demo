# cybOS Telemetry & Observability

## Purpose

Telemetry is an on-device observability surface for the cybOS native application. It should answer:

- What is running, waiting, failing, or exceeding its budget?
- Which host processes currently consume CPU and memory?
- How much RAM and disk capacity remain?
- What thermal and network counters does the operating system expose?
- Is a metric a live measurement, an app-reported status, or not instrumented yet?

Telemetry must not invent readings, run expensive collection on the UI thread, or upload host/process data.

## Current implementation (Phase 1)

The System Core page currently samples host metrics every 2 seconds and retains at most 60 samples in memory:

- Global CPU utilization and logical CPU count.
- Used/total RAM and process count.
- Top eight processes by reported CPU, with PID and memory use.
- Total/free disk capacity across volumes reported by the OS.
- OS network received/transmitted counters.
- Highest available OS-reported temperature and the number of exposed sensors.
- CPU/RAM sparklines, runtime-cell run counts, budgets, status and overruns.
- Bounded worker execution traces with correlation IDs, start timestamps, elapsed duration, budget and final status (up to 100 active/recent tasks).
- Export of telemetry history/current sample in the existing local JSON state export.

Sampling is local-only. The module does not send telemetry to any service. History is bounded and in-memory unless the user explicitly exports a state file. Process CPU readings are OS snapshots and may fluctuate between samples. Network counters are OS counters, not per-cybOS traffic attribution.

## Known limitations (not yet implemented)

- **Task/dispatcher detail:** worker execution spans are now recorded from `WorkerContract` starts and finishes, but individual task queue wait, enqueue/dequeue timestamps, retry counts, cancellation reasons, worker CPU attribution, and per-task memory peaks still need instrumentation at the dispatch boundary.
- **Video:** the current Cameras UI does not yet expose a connected decode pipeline. FPS, dropped frames, decode latency, buffer depth, codec, GPU video engine utilization, and VRAM are therefore not reported.
- **Maps:** Offline Atlas currently inspects MBTiles archives. There is no map renderer/tile cache pipeline to measure yet; tile-cache hit rate, tile decode time, frame/render latency, and GPU draw cost should be added with the renderer.
- **GPU/thermal:** portable GPU utilization, VRAM, fan speed, and component temperatures differ by OS and hardware. Add platform adapters (for example, macOS-specific APIs and supported Windows/Linux providers) and clearly label unsupported sensors. Never replace missing data with a simulated value.
- **Alerts and persistence:** configurable thresholds, incident timeline, longer-term retention, and CSV/JSONL export are future work.

## Recommended next phases

1. **Dispatcher spans:** assign each task a correlation ID; record created, queued, started, finished, cancelled and failed timestamps; record queue depth, retry count, budget, exit reason, and worker heartbeat.
2. **Worker resource attribution:** track per-worker elapsed time and process/task context where the OS allows it. Distinguish measured host load from estimated task cost.
3. **Media instrumentation:** when a real decoder is integrated, expose frames received/decoded/dropped, rolling FPS, decode latency percentiles, buffer depth and hardware-acceleration status.
4. **Map instrumentation:** after map rendering exists, expose tile-cache hit/miss, tile fetch/decode latency, render duration, visible tile count and memory budget.
5. **Hardware adapters:** add platform-specific GPU/VRAM, thermal and fan metrics only through supported APIs, with source and availability metadata.
6. **Incidents and retention:** add configurable local alerts, bounded persistent history, retention controls and explicit export.

## Design rules

- Show source, timestamp and availability for each metric.
- Distinguish zero from unknown/unsupported.
- Keep sampling bounded and avoid blocking network/model work in the UI thread.
- Use a bounded ring buffer and explicit retention limits.
- Never include private keys, seed phrases, message bodies or credentials in telemetry exports.
- Keep collection local by default; any future remote reporting must be opt-in and documented.
