# cybOS user experience and interaction guide

## Product rule

Every screen must answer three questions without requiring prior knowledge:
1. **What is this?** A short purpose statement appears in the shared Quick Guide.
2. **What should I do first?** The “HOW TO USE” disclosure gives one safe first step.
3. **What should I be careful about?** The guide names the important limitation for that screen.

## Global navigation

- The left rail opens the primary workspaces. Hover over an icon to see its label.
- The top search accepts page names and supported keywords; press **⌘K** to focus it, then **Enter** or **GO** to navigate.
- The bottom shortcuts provide quick access to common workspaces.
- The page title and status remain visible above the content.
- The Quick Guide is shown consistently on every page; expand **HOW TO USE** for the first step and the key limitation.

## Screen-by-screen first steps

| Screen | First step | Important limitation |
|---|---|---|
| Dashboard | Choose a labelled module | Demo cards are not automatically live sensor readings |
| RobotCYB | Enter a request; define acceptance criteria for repeatable work | A budget ceiling does not authorize payments or physical actions |
| CicadaFarm | Review farm state and record observations | Only explicitly connected readings are live measurements |
| CybChat | Start local chat, or discover a peer for peer messaging | Verify first-seen peer identity out of band |
| Graph | Select a node and inspect its links | Stored links are not proof of truth |
| Brain | Check local Qwen status | AI requests may not run while Qwen is offline |
| Network | Scan and inspect discovered peers | Discovery does not imply trust |
| Radar | Opt in to visibility before advertising the node | Nearby visibility can expose presence |
| Assets | Review addresses and displayed balances | Never enter a seed phrase or private key |
| Cameras | Select a zone and check connection status | A configured camera may be offline |
| CybLex | Verify source and destination before a job | Share only authorized content |
| CybDex | Search a mint or symbol, then select a pair | Read-only market data; no swaps are signed |
| CybBrowser | Enter a full URL and press Open | Remote JavaScript is not executed; remote content is untrusted |
| System | Review ERROR/OFFLINE components | READY is a local status, not a guarantee of external reachability |

## Interaction and accessibility checklist

- Use descriptive button text rather than icon-only actions where space permits.
- Every icon-only navigation action must have a hover label.
- Use plain-language validation and error messages; do not silently discard invalid input.
- Disable actions while their worker is busy, and show a visible status/result after completion.
- Confirm risky, irreversible, financial, network-sharing, or physical actions before execution.
- Clearly distinguish **local**, **connected**, **offline**, **demo**, and **live** data.
- Keep controls grouped by task, with the expected sequence visible.
- Preserve the cyberpunk visual identity without sacrificing readable contrast or legible text.
- Never label an unimplemented capability as working; use “planned”, “offline”, or “not connected” honestly.

## Definition of done for a screen

A screen is ready for release only when its primary task can be completed by a first-time user without guessing; each control has a clear label or tooltip; empty, busy, success, and error states are understandable; and limitations are visible before a user relies on the output.
