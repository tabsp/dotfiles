import { useCallback, useEffect, useRef, useState } from "react";

const CANONICAL_WIDTH = 1188;
const CANONICAL_HEIGHT = 750;
const FONT_SIZE = 13;
const FONT_FAMILY = '"Dotman Maple Mono", monospace';
const RUN_STEP_DELAYS = [900, 1100, 1000, 1200, 1100, 1400];
const KEY_CODES = {
  ArrowUp: 1,
  k: 1,
  ArrowDown: 2,
  j: 2,
  ArrowLeft: 3,
  h: 3,
  ArrowRight: 4,
  l: 4,
  Enter: 5,
  " ": 6,
  q: 7,
  r: 8,
  Escape: 9,
  PageUp: 10,
  PageDown: 11,
  Home: 12,
  End: 13,
  a: 14,
  n: 15,
  s: 16,
  Tab: 17,
  f: 17,
  d: 20,
  p: 21,
  D: 23,
  1: 31,
  2: 32,
  3: 33,
  4: 34,
  5: 35,
  6: 36,
};
const HANDLED_KEYS = Object.keys(KEY_CODES);
const SCREEN_NAMES = [
  "main-menu",
  "plan",
  "review",
  "run",
  "result",
  "history",
  "replay",
];
const TOUCH_CONTROL = {
  up: { key: "ArrowUp", label: "↑", ariaLabel: "Move up" },
  down: { key: "ArrowDown", label: "↓", ariaLabel: "Move down" },
  enter: { key: "Enter", label: "Enter", ariaLabel: "Open or confirm" },
  space: { key: " ", label: "Space", ariaLabel: "Toggle selection" },
  run: { key: "r", label: "r", ariaLabel: "Review or run" },
  back: { key: "q", label: "q", ariaLabel: "Go back" },
  save: { key: "s", label: "Save", ariaLabel: "Save selection" },
  discard: {
    key: "D",
    label: "Discard",
    ariaLabel: "Discard selection changes",
  },
  cancel: { key: "Escape", label: "Cancel", ariaLabel: "Cancel going back" },
  filter: { key: "Tab", label: "Filter", ariaLabel: "Cycle log filter" },
  fold: { key: "Enter", label: "Fold", ariaLabel: "Toggle log folding" },
};
const TOUCH_KEYS_BY_SCREEN = {
  "main-menu": [
    TOUCH_CONTROL.up,
    TOUCH_CONTROL.down,
    TOUCH_CONTROL.enter,
    TOUCH_CONTROL.back,
  ],
  plan: [
    TOUCH_CONTROL.up,
    TOUCH_CONTROL.down,
    TOUCH_CONTROL.space,
    TOUCH_CONTROL.run,
    TOUCH_CONTROL.back,
  ],
  review: [
    TOUCH_CONTROL.up,
    TOUCH_CONTROL.down,
    TOUCH_CONTROL.run,
    TOUCH_CONTROL.back,
  ],
  run: [TOUCH_CONTROL.filter, TOUCH_CONTROL.fold, TOUCH_CONTROL.back],
  result: [TOUCH_CONTROL.filter, TOUCH_CONTROL.fold, TOUCH_CONTROL.back],
  history: [
    TOUCH_CONTROL.up,
    TOUCH_CONTROL.down,
    TOUCH_CONTROL.enter,
    TOUCH_CONTROL.back,
  ],
  replay: [
    TOUCH_CONTROL.up,
    TOUCH_CONTROL.down,
    TOUCH_CONTROL.space,
    TOUCH_CONTROL.back,
  ],
};
const FRAME_CACHE = new WeakMap();

function numberedFrameCount(bundle, prefix) {
  const pattern = new RegExp(`^${prefix}-(\\d+)$`);
  return bundle.frames.reduce(
    (count, frame) => (pattern.test(frame.id) ? count + 1 : count),
    0,
  );
}

function replayFrameCount(bundle, runIndex) {
  const pattern = new RegExp(`^replay-${runIndex}-(\\d+)$`);
  return bundle.frames.reduce(
    (count, frame) => (pattern.test(frame.id) ? count + 1 : count),
    0,
  );
}

function configureEngine(engine, bundle) {
  const runIndex = engine.demo_parent_index();
  engine.demo_configure(
    0,
    0,
    numberedFrameCount(bundle, "history"),
    replayFrameCount(bundle, runIndex),
  );
}

function frameIdFromEngine(engine) {
  const screen = engine.demo_screen();
  const index = engine.demo_index();
  switch (screen) {
    case 0:
      return `main-menu-${index}`;
    case 1:
      return `plan-${index}`;
    case 2:
      return `review-${index}`;
    case 3:
      return ["run-0", "run-3", "run-6"][engine.demo_run_step()] || "run-6";
    case 4:
      return "result";
    case 5:
      return `history-${index}`;
    case 6:
      return `replay-${engine.demo_parent_index()}-${index}`;
    default:
      return "main-menu-0";
  }
}

function initializeEngineSeed(engine, seed, width, height) {
  const bytes = new TextEncoder().encode(JSON.stringify(seed));
  const pointer = engine.demo_alloc(bytes.length);
  new Uint8Array(engine.memory.buffer, pointer, bytes.length).set(bytes);
  if (engine.demo_init_seed(pointer, bytes.length, width, height) !== 1) {
    throw new Error("WASM rejected the generated demo seed");
  }
}

function dynamicFrameFromEngine(engine) {
  if (!engine || ![1, 2, 3, 4].includes(engine.demo_screen())) return null;
  engine.demo_render();
  const pointer = engine.demo_output_ptr();
  const length = engine.demo_output_len();
  const bytes = new Uint8Array(engine.memory.buffer, pointer, length).slice();
  return JSON.parse(new TextDecoder().decode(bytes));
}

function keyCodeForEngine(key, screen) {
  if (key === "h" && screen === 0) return 22;
  return KEY_CODES[key];
}

function useDemoRuntime() {
  const [bundle, setBundle] = useState(null);
  const [engine, setEngine] = useState(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    Promise.all([fetch("/demo-frames.json"), fetch("/dotman-web-state.wasm")])
      .then(async ([frameResponse, wasmResponse]) => {
        if (!frameResponse.ok)
          throw new Error(`frame data returned ${frameResponse.status}`);
        if (!wasmResponse.ok)
          throw new Error(`WASM state machine returned ${wasmResponse.status}`);
        const [data, wasmBytes] = await Promise.all([
          frameResponse.json(),
          wasmResponse.arrayBuffer(),
        ]);
        const { instance } = await WebAssembly.instantiate(wasmBytes, {});
        initializeEngineSeed(
          instance.exports,
          data.seed,
          data.width,
          data.height,
        );
        configureEngine(instance.exports, data);
        return { data, engine: instance.exports };
      })
      .then((runtime) => {
        if (active) {
          setBundle(runtime.data);
          setEngine(runtime.engine);
        }
      })
      .catch((cause) => {
        if (active) setError(cause.message);
      });
    return () => {
      active = false;
    };
  }, []);

  return { bundle, engine, error };
}

function fontFor(cell) {
  const weight = cell.bold ? 700 : 500;
  const style = cell.italic ? "italic" : "normal";
  return `${style} ${weight} ${FONT_SIZE}px ${FONT_FAMILY}`;
}

function drawFrame(canvas, bundle, frame) {
  if (!canvas || !bundle || !frame) return;

  const ratio = Math.max(1, window.devicePixelRatio || 1);
  canvas.width = Math.round(CANONICAL_WIDTH * ratio);
  canvas.height = Math.round(CANONICAL_HEIGHT * ratio);
  canvas.style.aspectRatio = `${CANONICAL_WIDTH} / ${CANONICAL_HEIGHT}`;

  const context = canvas.getContext("2d", { alpha: false });
  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  context.textBaseline = "alphabetic";
  context.imageSmoothingEnabled = false;
  context.fillStyle = bundle.default_bg;
  context.fillRect(0, 0, CANONICAL_WIDTH, CANONICAL_HEIGHT);

  const cellWidth = CANONICAL_WIDTH / bundle.width;
  const cellHeight = CANONICAL_HEIGHT / bundle.height;
  const baseline = (cellHeight - FONT_SIZE) / 2 + FONT_SIZE * 0.82;

  // Paint every cell background first. Nerd Font glyphs can overhang their
  // nominal cell, and a later background fill must not cover that overhang.
  for (const cell of frame.cells) {
    const x = cell.x * cellWidth;
    const y = cell.y * cellHeight;
    const background = cell.reversed
      ? cell.fg || bundle.default_fg
      : cell.bg || bundle.default_bg;

    if (background !== bundle.default_bg || cell.reversed) {
      context.fillStyle = background;
      context.fillRect(
        Math.floor(x),
        Math.floor(y),
        Math.ceil(cellWidth + 0.25),
        Math.ceil(cellHeight + 0.25),
      );
    }
  }

  // Draw glyphs as a second pass so terminal icons keep their full bearings.
  for (const cell of frame.cells) {
    if (!cell.symbol || cell.symbol === " ") continue;

    const x = cell.x * cellWidth;
    const y = cell.y * cellHeight;
    const foreground = cell.reversed
      ? cell.bg || bundle.default_bg
      : cell.fg || bundle.default_fg;

    context.save();
    context.globalAlpha = cell.dim ? 0.62 : 1;
    context.font = fontFor(cell);
    context.fillStyle = foreground;
    context.fillText(cell.symbol, x, y + baseline);
    if (cell.underlined) {
      context.fillRect(x, y + cellHeight - 2, cellWidth, 1);
    }
    context.restore();
  }
}

function frameById(bundle, id) {
  if (!bundle) return null;
  let cache = FRAME_CACHE.get(bundle);
  if (!cache) {
    cache = new Map();
    FRAME_CACHE.set(bundle, cache);
  }
  if (cache.has(id)) return cache.get(id);

  const frame = bundle.frames.find((candidate) => candidate.id === id);
  if (!frame) return null;
  if (!frame.base) {
    cache.set(id, frame);
    return frame;
  }

  const base = frameById(bundle, frame.base);
  if (!base) return null;
  const cells = new Map(
    base.cells.map((cell) => [`${cell.x}:${cell.y}`, cell]),
  );
  for (const cell of frame.cells) {
    const key = `${cell.x}:${cell.y}`;
    const isBlank =
      cell.symbol === " " &&
      !cell.fg &&
      !cell.bg &&
      !cell.bold &&
      !cell.dim &&
      !cell.italic &&
      !cell.underlined &&
      !cell.reversed;
    if (isBlank) cells.delete(key);
    else cells.set(key, cell);
  }
  const materialized = { ...frame, cells: [...cells.values()] };
  cache.set(id, materialized);
  return materialized;
}

function expandedReplayFrame(bundle, frame) {
  if (!bundle || !frame) return frame;
  const focusY = frame.cells
    .filter((cell) => cell.symbol === "▎")
    .reduce((latest, cell) => Math.max(latest, cell.y), -1);
  if (focusY < 0) return frame;
  const footerY = frame.cells
    .filter((cell) => cell.symbol === "[" && cell.y > focusY)
    .reduce((earliest, cell) => Math.min(earliest, cell.y), bundle.height - 1);
  const cells = frame.cells
    .filter((cell) => !(cell.y === footerY - 1 && cell.y > focusY))
    .map((cell) =>
      cell.y > focusY && cell.y < footerY - 1
        ? { ...cell, y: cell.y + 1 }
        : cell,
    );
  const detail = "    no saved output";
  for (let offset = 0; offset < detail.length; offset += 1) {
    cells.push({
      x: 1 + offset,
      y: focusY + 1,
      symbol: detail[offset],
      fg: "#6c7086",
    });
  }
  return { ...frame, cells };
}

function frameText(bundle, frame) {
  if (!bundle || !frame) return "dotman UI loading";
  const rows = Array.from({ length: bundle.height }, () =>
    Array.from({ length: bundle.width }, () => " "),
  );
  for (const cell of frame.cells) {
    rows[cell.y][cell.x] = cell.symbol;
  }
  return rows
    .map((row) => row.join("").trimEnd())
    .join("\n")
    .trim();
}

export function App() {
  const { bundle, engine, error } = useDemoRuntime();
  const shellRef = useRef(null);
  const canvasRef = useRef(null);
  const timerRef = useRef(null);
  const [isRendered, setIsRendered] = useState(false);
  const [runtimeVersion, setRuntimeVersion] = useState(0);
  const debugFrameId = new URLSearchParams(window.location.search).get("frame");
  const frameId =
    debugFrameId || (engine ? frameIdFromEngine(engine) : "main-menu-0");
  const dynamicFrame =
    engine && runtimeVersion >= 0 ? dynamicFrameFromEngine(engine) : null;
  const staticFrame = frameById(bundle, frameId);
  const replayFrame =
    engine?.demo_screen() === 6 && engine.demo_replay_expanded() === 1
      ? expandedReplayFrame(bundle, staticFrame)
      : staticFrame;
  const activeFrame = dynamicFrame || replayFrame;
  const activeScreen =
    activeFrame?.screen ||
    (engine ? SCREEN_NAMES[engine.demo_screen()] : "loading");
  const touchKeys =
    activeScreen === "plan" && engine?.demo_exit_pending() === 1
      ? [TOUCH_CONTROL.save, TOUCH_CONTROL.discard, TOUCH_CONTROL.cancel]
      : TOUCH_KEYS_BY_SCREEN[activeScreen] || [];

  const stopRunAnimation = useCallback(() => {
    if (timerRef.current) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const startRunAnimation = useCallback(() => {
    if (!engine) return;
    stopRunAnimation();
    const next = () => {
      engine.demo_tick();
      setRuntimeVersion((version) => version + 1);
      if (engine.demo_screen() === 3) {
        const step = Math.min(
          engine.demo_run_step(),
          RUN_STEP_DELAYS.length - 1,
        );
        timerRef.current = window.setTimeout(next, RUN_STEP_DELAYS[step]);
      }
    };
    timerRef.current = window.setTimeout(next, RUN_STEP_DELAYS[0]);
  }, [engine, stopRunAnimation]);

  useEffect(() => () => stopRunAnimation(), [stopRunAnimation]);

  useEffect(() => {
    if (!bundle) return;
    const frame = activeFrame;
    let cancelled = false;
    const render = () => {
      if (!cancelled) {
        drawFrame(canvasRef.current, bundle, frame);
        setIsRendered(true);
      }
    };
    const loadFontsAndRender = async () => {
      await Promise.all([
        document.fonts.load(`500 ${FONT_SIZE}px ${FONT_FAMILY}`),
        document.fonts.load(`700 ${FONT_SIZE}px ${FONT_FAMILY}`),
      ]);
      render();
    };
    loadFontsAndRender().catch(render);
    window.addEventListener("resize", render);
    document.fonts.addEventListener("loadingdone", render);
    return () => {
      cancelled = true;
      window.removeEventListener("resize", render);
      document.fonts.removeEventListener("loadingdone", render);
    };
  }, [activeFrame, bundle]);

  const handleInput = useCallback(
    (key) => {
      if (!bundle || !engine || debugFrameId) return;
      const keyCode = keyCodeForEngine(key, engine.demo_screen());
      if (!keyCode) return;
      const previousScreen = engine.demo_screen();
      configureEngine(engine, bundle);
      engine.demo_input(keyCode);
      configureEngine(engine, bundle);
      setRuntimeVersion((version) => version + 1);
      if (previousScreen !== 3 && engine.demo_screen() === 3)
        startRunAnimation();
      if (previousScreen === 3 && engine.demo_screen() !== 3)
        stopRunAnimation();
    },
    [bundle, debugFrameId, engine, startRunAnimation, stopRunAnimation],
  );

  const handleKeyDown = useCallback(
    (event) => {
      const target = event.target;
      if (
        target instanceof HTMLElement &&
        (target.isContentEditable ||
          ["BUTTON", "INPUT", "SELECT", "TEXTAREA"].includes(target.tagName))
      ) {
        return;
      }
      if (HANDLED_KEYS.includes(event.key)) event.preventDefault();
      handleInput(event.key);
    },
    [handleInput],
  );

  useEffect(() => {
    if (!bundle || !engine || debugFrameId) return undefined;
    const focusFrame = window.requestAnimationFrame(() => {
      shellRef.current?.focus({ preventScroll: true });
    });
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.cancelAnimationFrame(focusFrame);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [bundle, debugFrameId, engine, handleKeyDown]);

  const handleTouchKey = useCallback(
    (key) => {
      handleInput(key);
      window.requestAnimationFrame(() =>
        shellRef.current?.focus({ preventScroll: true }),
      );
    },
    [handleInput],
  );

  const handlePointer = useCallback(
    (event) => {
      if (!bundle || !engine || debugFrameId) return;
      const rect = event.currentTarget.getBoundingClientRect();
      const column = Math.max(
        0,
        Math.min(
          bundle.width - 1,
          Math.floor(((event.clientX - rect.left) / rect.width) * bundle.width),
        ),
      );
      const row = Math.max(
        0,
        Math.min(
          bundle.height - 1,
          Math.floor(
            ((event.clientY - rect.top) / rect.height) * bundle.height,
          ),
        ),
      );
      const previousScreen = engine.demo_screen();
      configureEngine(engine, bundle);
      engine.demo_pointer(column, row, event.detail);
      configureEngine(engine, bundle);
      setRuntimeVersion((version) => version + 1);
      if (previousScreen !== 3 && engine.demo_screen() === 3)
        startRunAnimation();
      if (previousScreen === 3 && engine.demo_screen() !== 3)
        stopRunAnimation();
      shellRef.current?.focus({ preventScroll: true });
    },
    [bundle, debugFrameId, engine, startRunAnimation, stopRunAnimation],
  );

  const handleWheel = useCallback(
    (event) => {
      if (event.shiftKey || Math.abs(event.deltaX) > Math.abs(event.deltaY)) {
        return;
      }
      event.preventDefault();
      handleInput(event.deltaY < 0 ? "ArrowUp" : "ArrowDown");
      shellRef.current?.focus({ preventScroll: true });
    },
    [handleInput],
  );

  if (error) {
    return (
      <main className="boot-message">dotman demo unavailable: {error}</main>
    );
  }

  return (
    <main
      ref={shellRef}
      className="demo-shell mobile-prototype"
      tabIndex={0}
      autoFocus
      aria-label="Interactive dotman TUI demonstration. This demo never runs commands or changes your machine."
      aria-describedby="frame-transcript"
    >
      {(!bundle || !engine) && (
        <p className="boot-message">loading dotman UI…</p>
      )}
      <div className="terminal-viewport">
        <div className="terminal-pan">
          <canvas
            ref={canvasRef}
            className={
              isRendered ? "terminal-canvas is-ready" : "terminal-canvas"
            }
            role="img"
            aria-label={`dotman ${activeFrame?.screen || "loading"} screen`}
            onClick={handlePointer}
            onWheel={handleWheel}
          />
        </div>
      </div>
      <div
        className="touch-controls"
        role="group"
        aria-label="dotman mobile controls"
        style={{ "--control-count": touchKeys.length }}
      >
        {touchKeys.map((control) => (
          <button
            key={control.key}
            type="button"
            className="touch-key"
            aria-label={control.ariaLabel}
            onClick={() => handleTouchKey(control.key)}
          >
            {control.label}
          </button>
        ))}
      </div>
      <pre id="frame-transcript" className="visually-hidden">
        {frameText(bundle, activeFrame)}
      </pre>
      <p className="visually-hidden" aria-live="polite">
        Current demo state: {activeScreen}. No commands are executed.
      </p>
    </main>
  );
}
