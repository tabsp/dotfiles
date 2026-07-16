# Prototype Instructions

Run the local server yourself and open the preview in the browser available to this environment. Do not give the user server-start instructions when you can run it.

Before making substantial visual changes, use the Product Design plugin's `get-context` skill when the visual source is unclear or no longer matches the current goal. When the user gives durable prototype-specific design feedback, preferences, or decisions, record them in `AGENTS.md`.

When implementing from a selected generated mock, treat that image as the source of truth for layout, component anatomy, density, spacing, color, typography, visible content, and hierarchy.

## dotman web demo constraints

- The sole UI source of truth is the real Ratatui renderers and interaction code under `../src/tui/`, together with the tokens in `../src/theme.rs`. Generate browser-visible states from Ratatui buffers; do not reconstruct layouts by tracing screenshots.
- `../assets/screenshots/dotman-main-menu.png` is secondary evidence only: use it to calibrate terminal font metrics, the canonical viewport, and final visual QA. It must not define markup, layout structure, content, or state behavior.
- Reproduce the dotman TUI only. Do not add a conventional landing page, hero, marketing cards, or unrelated navigation.
- Cover Main Menu, Plan, Review, and simulated Run/Result. The browser must never execute deployment, shell, filesystem, or package-management actions.
- Preserve the real keyboard model where represented: arrows/j/k, Enter, Space, r, q, Tab, and f.
- Keyboard shortcuts must work immediately after runtime readiness without requiring a pointer click. Focus the shell after the frame/WASM bundle loads and keep page-level shortcuts safe around buttons and editable controls.
- Keep the simulated Run sequence readable: the six progress/log stages should take roughly 6–7 seconds in total, with each state visible for about one second.
- Preserve mouse parity without DOM overlays: translate Canvas pointer coordinates into terminal cells, route clicks through WASM, and use vertical wheel input for TUI navigation/scrolling. Keep horizontal/Shift-wheel gestures available to the portrait terminal viewport.
- Self-hosted font subsets must include glyphs emitted only by dynamic WASM states, not just generated baseline frames. In particular, keep the collapsed-layer and unchecked-selection Nerd Font glyphs.
- Target pixel-level fidelity at a canonical 1188 x 803 CSS-pixel viewport (the reference screenshot is a 2x 2376 x 1606 capture). Scale the terminal surface as a unit at other viewport sizes rather than redesigning it.
- Use the Catppuccin Mocha values from `../src/theme.rs`; do not invent replacement colors.
- Build output is disposable and must not be committed. The site is intended for Vercel static hosting.
