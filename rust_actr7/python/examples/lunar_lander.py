#!/usr/bin/env python3
"""Lunar lander: a playable example of the Rust ACT-R Python module API.

Human mode:
    python examples/lunar_lander.py --mode human

Model mode:
    python examples/lunar_lander.py --mode model --seed 1
    python examples/lunar_lander.py --mode model --seed 1 --gui

The game has three independent thrusters: main (up), left, and right. You can
hold several at once. The first thrust starts the run. Land on the green pad
without hitting the terrain or arriving too fast.
"""

from __future__ import annotations

import argparse
import math
import random
import sys
import time
from dataclasses import dataclass
from typing import Any, Optional

try:
    import tkinter as tk
except ImportError:  # pragma: no cover - human mode needs tkinter
    tk = None

try:
    import rust_actr as actr
except ImportError:  # human mode still works without the wheel
    actr = None


WIDTH = 900
HEIGHT = 600
PAD_WIDTH = 160
GROUND_BASE = HEIGHT - 80
LANDER_W = 22
LANDER_H = 26
GRAVITY = 70.0
MAIN_ACCEL = 155.0
SIDE_ACCEL = 95.0
FUEL_MAX = 100.0
MAIN_BURN = 18.0
SIDE_BURN = 10.0
SAFE_VY = 60.0
SAFE_VX = 30.0
TICK_MS = 50
FPS = 60


def clamp(value: float, low: float, high: float) -> float:
    return max(low, min(high, value))


@dataclass
class Terrain:
    points: list[tuple[float, float]]
    pad_left: float
    pad_right: float
    pad_y: float

    @classmethod
    def generate(cls, rng: random.Random) -> "Terrain":
        pad_center = rng.uniform(WIDTH * 0.25, WIDTH * 0.75)
        pad_left = pad_center - PAD_WIDTH / 2
        pad_right = pad_center + PAD_WIDTH / 2
        pad_y = GROUND_BASE
        xs = {0.0, float(WIDTH), pad_left, pad_right}
        xs.update(float(x) for x in range(0, WIDTH + 1, 24))
        points: list[tuple[float, float]] = []
        for x in sorted(xs):
            if pad_left <= x <= pad_right:
                y = pad_y
            else:
                y = rng.uniform(GROUND_BASE - 34, GROUND_BASE + 42)
                y = clamp(y, 120.0, HEIGHT - 18.0)
            points.append((x, y))
        return cls(points, pad_left, pad_right, pad_y)

    def height_at(self, x: float) -> float:
        if x <= self.points[0][0]:
            return self.points[0][1]
        if x >= self.points[-1][0]:
            return self.points[-1][1]
        for (x0, y0), (x1, y1) in zip(self.points, self.points[1:]):
            if x0 <= x <= x1:
                if x1 == x0:
                    return y1
                t = (x - x0) / (x1 - x0)
                return y0 + (y1 - y0) * t
        return self.points[-1][1]

    def on_pad(self, x: float) -> bool:
        return self.pad_left <= x <= self.pad_right


class LunarLander:
    def __init__(self, seed: int = 0) -> None:
        self.seed = seed
        self.rng = random.Random(seed)
        self.terrain = Terrain.generate(self.rng)
        self.x = 0.0
        self.y = 0.0
        self.vx = 0.0
        self.vy = 0.0
        self.fuel = FUEL_MAX
        self.main = False
        self.left = False
        self.right = False
        self.status = "ready"
        self.elapsed = 0.0
        self.reset()

    def reset(self) -> None:
        self.terrain = Terrain.generate(self.rng)
        self.x = self.rng.uniform(WIDTH * 0.15, WIDTH * 0.85)
        self.y = self.rng.uniform(70.0, 150.0)
        self.vx = self.rng.uniform(-24.0, 24.0)
        self.vy = self.rng.uniform(0.0, 12.0)
        self.fuel = FUEL_MAX
        self.main = self.left = self.right = False
        self.status = "ready"
        self.elapsed = 0.0

    def start(self) -> None:
        if self.status == "ready":
            self.status = "flying"

    def set_thrusters(
        self,
        main: Optional[bool] = None,
        left: Optional[bool] = None,
        right: Optional[bool] = None,
    ) -> None:
        if main is not None:
            self.main = bool(main)
        if left is not None:
            self.left = bool(left)
        if right is not None:
            self.right = bool(right)
        if (self.main or self.left or self.right) and self.status == "ready":
            self.start()

    def altitude(self) -> float:
        return self.terrain.height_at(self.x) - (self.y + LANDER_H / 2)

    def step(self, dt: float) -> None:
        if self.status != "flying":
            return
        dt = clamp(dt, 0.0, 0.1)
        if dt <= 0.0:
            return
        self.elapsed += dt

        ax = 0.0
        ay = GRAVITY
        fuel_used = 0.0
        if self.fuel > 0.0:
            if self.main:
                ay -= MAIN_ACCEL
                fuel_used += MAIN_BURN * dt
            if self.left:
                ax -= SIDE_ACCEL
                fuel_used += SIDE_BURN * dt
            if self.right:
                ax += SIDE_ACCEL
                fuel_used += SIDE_BURN * dt
        self.fuel = max(0.0, self.fuel - fuel_used)
        if self.fuel <= 0.0:
            self.main = self.left = self.right = False

        self.vx += ax * dt
        self.vy += ay * dt
        self.x += self.vx * dt
        self.y += self.vy * dt

        if self.x < 0.0 or self.x > WIDTH:
            self.status = "crashed"
            self.main = self.left = self.right = False
            return

        ground = self.terrain.height_at(self.x)
        if self.y + LANDER_H / 2 >= ground:
            self.y = ground - LANDER_H / 2
            safe = (
                self.terrain.on_pad(self.x)
                and self.vy >= 0.0
                and self.vy <= SAFE_VY
                and abs(self.vx) <= SAFE_VX
            )
            self.status = "landed" if safe else "crashed"
            self.main = self.left = self.right = False

    def observation(self) -> dict[str, Any]:
        altitude_px = self.altitude()
        time_to_ground = clamp(altitude_px / max(20.0, self.vy), 0.35, 3.0)
        predicted_x = self.x + self.vx * time_to_ground
        if predicted_x < self.terrain.pad_left + 20:
            horizontal = "left"
        elif predicted_x > self.terrain.pad_right - 20:
            horizontal = "right"
        else:
            horizontal = "aligned"

        if self.vy > 45.0:
            vertical = "fast"
        elif self.vy < -2.0:
            vertical = "rising"
        else:
            vertical = "ok"

        altitude = "low" if altitude_px < 120.0 else "high"
        if self.fuel <= 0.0:
            fuel = "empty"
        elif self.fuel < 20.0:
            fuel = "low"
        else:
            fuel = "ok"

        return {
            "status": self.status.upper(),
            "vertical": vertical.upper(),
            "horizontal": horizontal.upper(),
            "altitude": altitude.upper(),
            "fuel": fuel.upper(),
            "x": round(self.x, 1),
            "y": round(self.y, 1),
            "vx": round(self.vx, 1),
            "vy": round(self.vy, 1),
        }


# --------------------------------------------------------------------------
# Python module for ACT-R models
# --------------------------------------------------------------------------

if actr is not None:

    class LanderModule(actr.Module):
        version = "0.1"
        documentation = "Lunar lander physics and buffer bridge"

        def __init__(self, lander: LunarLander, tick_ms: int = TICK_MS) -> None:
            self.lander = lander
            self.tick_ms = tick_ms
            self.last_tick_ms = 0

        def name(self) -> str:
            return "lander"

        def buffers(self) -> list[Any]:
            return [
                actr.BufferSpec("lander-state"),
                actr.BufferSpec(
                    "lander-control",
                    request_params=["main", "left", "right", "coast"],
                ),
            ]

        def chunk_types(self) -> list[Any]:
            return [
                actr.ChunkTypeSpec(
                    "lander-state",
                    slots=[
                        "status",
                        "vertical",
                        "horizontal",
                        "altitude",
                        "fuel",
                        "x",
                        "y",
                        "vx",
                        "vy",
                    ],
                ),
                actr.ChunkTypeSpec(
                    "lander-control",
                    slots=["main", "left", "right", "coast"],
                ),
            ]

        def parameters(self) -> list[Any]:
            return [actr.ParamSpec(":lander-tick", default_value=self.tick_ms)]

        def create(self, ctx: Any) -> None:
            self.last_tick_ms = ctx.time_ms()
            self._publish(ctx)
            ctx.set_busy("lander-control", False)

        def reset(self, ctx: Any, phase: Any) -> None:
            if phase != actr.ResetPhase.Primary:
                return
            self.lander.reset()
            self.last_tick_ms = ctx.time_ms()
            self._publish(ctx)
            ctx.set_busy("lander-control", False)
            self.lander.start()
            ctx.emit("LAUNCH")
            ctx.schedule_named(self.tick_ms, "tick", [])

        def request(self, ctx: Any, buffer: str, spec: Any) -> None:
            if buffer != "LANDER-CONTROL":
                return
            slots = spec.slots
            if "COAST" in slots:
                self.lander.set_thrusters(False, False, False)
            else:
                self.lander.set_thrusters(
                    "MAIN" in slots,
                    "LEFT" in slots,
                    "RIGHT" in slots,
                )
            self.lander.start()
            ctx.set_busy("lander-control", True)

        def on_event(self, ctx: Any, event: Any) -> None:
            if event.name != "tick":
                return
            now = ctx.time_ms()
            dt = (now - self.last_tick_ms) / 1000.0
            self.last_tick_ms = now
            if self.lander.status == "flying":
                self.lander.step(dt if dt > 0.0 else self.tick_ms / 1000.0)
            self._publish(ctx)
            ctx.set_busy("lander-control", False)
            if self.lander.status == "flying":
                ctx.schedule_named(self.tick_ms, "tick", [])
            else:
                ctx.emit(
                    f"{self.lander.status.upper()} "
                    f"fuel={self.lander.fuel:.0f} time={self.lander.elapsed:.2f}"
                )

        def params(
            self, ctx: Any, name: str, value: Any = actr.UNSET
        ) -> Optional[Any]:
            if name.upper() == ":LANDER-TICK":
                if value is not actr.UNSET:
                    self.tick_ms = max(1, int(value))
                return self.tick_ms
            return None

        def _publish(self, ctx: Any) -> None:
            ctx.set_buffer(
                "lander-state", "lander-state", self.lander.observation()
            )


PRODUCTIONS = r"""
(p fuel-out
   =goal> state play
   =lander-state> status flying fuel empty
   ?lander-control> state free
   ==>
   +lander-control> coast t)

(p steer-right-fast
   =goal> state play
   =lander-state> status flying horizontal left vertical fast fuel ok
   ?lander-control> state free
   ==>
   +lander-control> main t right t)

(p steer-right-fast-low
   =goal> state play
   =lander-state> status flying horizontal left vertical fast fuel low
   ?lander-control> state free
   ==>
   +lander-control> main t right t)

(p steer-right
   =goal> state play
   =lander-state> status flying horizontal left - vertical fast fuel ok
   ?lander-control> state free
   ==>
   +lander-control> right t)

(p steer-right-low
   =goal> state play
   =lander-state> status flying horizontal left - vertical fast fuel low
   ?lander-control> state free
   ==>
   +lander-control> right t)

(p steer-left-fast
   =goal> state play
   =lander-state> status flying horizontal right vertical fast fuel ok
   ?lander-control> state free
   ==>
   +lander-control> main t left t)

(p steer-left-fast-low
   =goal> state play
   =lander-state> status flying horizontal right vertical fast fuel low
   ?lander-control> state free
   ==>
   +lander-control> main t left t)

(p steer-left
   =goal> state play
   =lander-state> status flying horizontal right - vertical fast fuel ok
   ?lander-control> state free
   ==>
   +lander-control> left t)

(p steer-left-low
   =goal> state play
   =lander-state> status flying horizontal right - vertical fast fuel low
   ?lander-control> state free
   ==>
   +lander-control> left t)

(p brake
   =goal> state play
   =lander-state> status flying horizontal aligned vertical fast fuel ok
   ?lander-control> state free
   ==>
   +lander-control> main t)

(p brake-low
   =goal> state play
   =lander-state> status flying horizontal aligned vertical fast fuel low
   ?lander-control> state free
   ==>
   +lander-control> main t)

(p descend
   =goal> state play
   =lander-state> status flying horizontal aligned vertical ok altitude high fuel ok
   ?lander-control> state free
   ==>
   +lander-control> coast t)

(p descend-low
   =goal> state play
   =lander-state> status flying horizontal aligned vertical ok altitude high fuel low
   ?lander-control> state free
   ==>
   +lander-control> coast t)

(p float
   =goal> state play
   =lander-state> status flying horizontal aligned vertical rising
   ?lander-control> state free
   ==>
   +lander-control> coast t)

(p low-coast
   =goal> state play
   =lander-state> status flying horizontal aligned - vertical fast altitude low
   ?lander-control> state free
   ==>
   +lander-control> coast t)
"""


def build_model(
    lander: LunarLander, seed: int, trace: bool = False
) -> Any:
    if actr is None:
        raise RuntimeError(
            "rust_actr is not installed. Build/install the Python wheel first: "
            "python -m maturin build --release && "
            "python -m pip install --force-reinstall "
            "target/wheels/rust_actr-*.whl"
        )
    model = actr.Model(
        "lunar-lander",
        seed=(seed, 0),
        modules=[lambda: LanderModule(lander)],
    )
    model.chunk_type("goal", "state")
    start = model.add_chunk("start", "goal", state="play")
    model.goal_focus(start)
    if not trace:
        model.sgp(":v", False)
    for text in PRODUCTIONS.strip().split("\n\n"):
        if text.strip():
            model.add_production(text.strip())
    return model


# --------------------------------------------------------------------------
# tkinter front end
# --------------------------------------------------------------------------


class LunarApp:
    def __init__(
        self,
        root: Any,
        lander: LunarLander,
        mode: str = "human",
        model: Any = None,
        fps: int = FPS,
        seed: int = 0,
    ) -> None:
        self.root = root
        self.lander = lander
        self.mode = mode
        self.model = model
        self.fps = max(10, fps)
        self.seed = seed
        self.pressed: set[str] = set()
        self.last_time = time.monotonic()
        self.model_done = False
        self.stars = self._make_stars()

        root.title(f"Lunar Lander (seed {seed})")
        root.resizable(False, False)
        self.canvas = tk.Canvas(
            root,
            width=WIDTH,
            height=HEIGHT,
            bg="#0b1020",
            highlightthickness=0,
        )
        self.canvas.pack()
        root.bind("<KeyPress>", self.on_key_press)
        root.bind("<KeyRelease>", self.on_key_release)
        root.protocol("WM_DELETE_WINDOW", root.destroy)

        self.render()
        if mode == "model":
            root.after(16, self.model_tick)
        else:
            root.after(16, self.human_tick)

    def _make_stars(self) -> list[tuple[float, float, int]]:
        rng = random.Random(12345)
        return [
            (rng.uniform(0, WIDTH), rng.uniform(0, HEIGHT - 100), rng.choice([1, 1, 2]))
            for _ in range(90)
        ]

    def on_key_press(self, event: Any) -> None:
        key = event.keysym.lower()
        if key == "escape":
            self.root.destroy()
            return
        if self.mode != "human":
            return
        if key == "r":
            self.lander.reset()
            self.pressed.clear()
            self.last_time = time.monotonic()
            return
        if key in {"w", "up", "space", "a", "left", "d", "right"}:
            self.pressed.add(key)
            self._apply_human_thrusters()

    def on_key_release(self, event: Any) -> None:
        if self.mode != "human":
            return
        key = event.keysym.lower()
        self.pressed.discard(key)
        self._apply_human_thrusters()

    def _apply_human_thrusters(self) -> None:
        main = bool(self.pressed & {"w", "up", "space"})
        left = bool(self.pressed & {"a", "left"})
        right = bool(self.pressed & {"d", "right"})
        if main or left or right:
            self.lander.start()
        self.lander.set_thrusters(main, left, right)

    def human_tick(self) -> None:
        now = time.monotonic()
        dt = now - self.last_time
        self.last_time = now
        self.lander.step(dt)
        self.render()
        self.root.after(max(1, int(1000 / self.fps)), self.human_tick)

    def model_tick(self) -> None:
        if self.model is None or self.model_done:
            self.render()
            return
        target = self.model.time + 0.05
        for _ in range(16):
            if not self.model.step():
                self.model_done = True
                break
            if self.model.time >= target:
                break
        self.render()
        self.root.after(16, self.model_tick)

    def render(self) -> None:
        c = self.canvas
        c.delete("all")

        for x, y, radius in self.stars:
            c.create_oval(
                x - radius,
                y - radius,
                x + radius,
                y + radius,
                fill="#c7d2fe",
                outline="",
            )

        ground = self.lander.terrain
        polygon = [(0.0, HEIGHT), (WIDTH, HEIGHT)]
        polygon.extend(reversed(ground.points))
        c.create_polygon(
            [coord for point in polygon for coord in point],
            fill="#33384a",
            outline="#7b849c",
            width=2,
        )
        c.create_line(
            ground.pad_left,
            ground.pad_y,
            ground.pad_right,
            ground.pad_y,
            fill="#4ade80",
            width=8,
        )
        c.create_text(
            (ground.pad_left + ground.pad_right) / 2,
            ground.pad_y - 18,
            text="LAND HERE",
            fill="#86efac",
            font=("Consolas", 12, "bold"),
        )

        x = self.lander.x
        y = self.lander.y
        c.create_polygon(
            x,
            y - LANDER_H / 2,
            x - LANDER_W / 2,
            y + LANDER_H / 2,
            x + LANDER_W / 2,
            y + LANDER_H / 2,
            fill="#e5e7eb",
            outline="#94a3b8",
            width=2,
        )
        c.create_oval(x - 5, y - 5, x + 5, y + 5, fill="#38bdf8", outline="")

        if self.lander.main and self.lander.fuel > 0:
            c.create_polygon(
                x - 5,
                y + LANDER_H / 2,
                x + 5,
                y + LANDER_H / 2,
                x,
                y + LANDER_H / 2 + 24,
                fill="#fb923c",
                outline="",
            )
        if self.lander.left and self.lander.fuel > 0:
            # Exhaust points right, so the reaction pushes the lander left.
            c.create_polygon(
                x + LANDER_W / 2,
                y - 4,
                x + LANDER_W / 2,
                y + 6,
                x + LANDER_W / 2 + 18,
                y + 1,
                fill="#fbbf24",
                outline="",
            )
        if self.lander.right and self.lander.fuel > 0:
            c.create_polygon(
                x - LANDER_W / 2,
                y - 4,
                x - LANDER_W / 2,
                y + 6,
                x - LANDER_W / 2 - 18,
                y + 1,
                fill="#fbbf24",
                outline="",
            )

        self._draw_hud()

        if self.lander.status == "ready":
            self._banner("Press W / A / D (or arrow keys) to start")
        elif self.lander.status == "landed":
            self._banner("LANDED!  Press R for a new run")
        elif self.lander.status == "crashed":
            self._banner("CRASHED.  Press R to try again")

    def _draw_hud(self) -> None:
        c = self.canvas
        status = self.lander.status.upper()
        status_color = {
            "READY": "#fbbf24",
            "FLYING": "#e2e8f0",
            "LANDED": "#4ade80",
            "CRASHED": "#f87171",
        }.get(status, "#e2e8f0")
        altitude = max(0.0, self.lander.altitude())
        fuel_ratio = clamp(self.lander.fuel / FUEL_MAX, 0.0, 1.0)

        c.create_rectangle(0, 0, WIDTH, 28, fill="#111827", outline="")
        c.create_text(
            14,
            14,
            anchor="w",
            text=status,
            fill=status_color,
            font=("Consolas", 12, "bold"),
        )
        c.create_text(
            145,
            14,
            anchor="w",
            text="FUEL",
            fill="#94a3b8",
            font=("Consolas", 12),
        )
        c.create_rectangle(198, 7, 398, 21, fill="#1e293b", outline="#94a3b8")
        c.create_rectangle(
            200,
            9,
            200 + 196 * fuel_ratio,
            19,
            fill="#4ade80" if fuel_ratio > 0.2 else "#f87171",
            outline="",
        )
        c.create_text(
            430,
            14,
            anchor="w",
            text=f"ALT {altitude:6.1f}",
            fill="#e2e8f0",
            font=("Consolas", 12),
        )

        if self.mode == "model":
            c.create_text(
                WIDTH - 14,
                14,
                anchor="e",
                text="ACT-R MODEL",
                fill="#fbbf24",
                font=("Consolas", 12, "bold"),
            )

    def _banner(self, text: str) -> None:
        self.canvas.create_rectangle(
            WIDTH / 2 - 270,
            HEIGHT / 2 - 24,
            WIDTH / 2 + 270,
            HEIGHT / 2 + 24,
            fill="#111827",
            outline="#fbbf24",
            width=2,
        )
        self.canvas.create_text(
            WIDTH / 2,
            HEIGHT / 2,
            text=text,
            fill="#fde68a",
            font=("Consolas", 14, "bold"),
        )


# --------------------------------------------------------------------------
# entry point
# --------------------------------------------------------------------------


def run_headless(lander: LunarLander, seed: int, max_time: float, verbose: bool) -> int:
    try:
        model = build_model(lander, seed, trace=verbose)
        trace = model.run(max_time)
    except Exception as exc:
        print(f"model mode failed: {exc}", file=sys.stderr)
        return 1

    status = lander.status
    print(f"seed   {seed}")
    print(f"status {status}")
    print(f"time   {lander.elapsed:.2f}s")
    print(f"fuel   {lander.fuel:.1f}")
    print(
        "state  "
        f"x={lander.x:.1f} y={lander.y:.1f} "
        f"vx={lander.vx:.1f} vy={lander.vy:.1f}"
    )
    if verbose and trace:
        print("\n".join(trace.splitlines()[-20:]))
    return 0 if status in {"landed", "crashed"} else 1


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Lunar lander ACT-R module example")
    parser.add_argument("--mode", choices=["human", "model"], default="human")
    parser.add_argument("--seed", type=int, default=None)
    parser.add_argument("--max-time", type=float, default=25.0)
    parser.add_argument("--gui", action="store_true", help="visualize model mode")
    parser.add_argument("--fps", type=int, default=FPS)
    parser.add_argument("--verbose", action="store_true", help="print model trace tail")
    args = parser.parse_args(argv)

    seed = args.seed if args.seed is not None else random.randrange(1, 1_000_000)
    lander = LunarLander(seed)

    if args.mode == "human":
        if tk is None:
            print("human mode requires tkinter", file=sys.stderr)
            return 2
        root = tk.Tk()
        LunarApp(root, lander, mode="human", fps=args.fps, seed=seed)
        root.mainloop()
        return 0

    if actr is None:
        print(
            "model mode requires rust_actr. Build/install the Python wheel first.",
            file=sys.stderr,
        )
        return 2

    if args.gui:
        if tk is None:
            print("--gui requires tkinter", file=sys.stderr)
            return 2
        try:
            model = build_model(lander, seed, trace=False)
        except Exception as exc:
            print(f"model mode failed: {exc}", file=sys.stderr)
            return 1
        root = tk.Tk()
        LunarApp(root, lander, mode="model", model=model, fps=args.fps, seed=seed)
        root.mainloop()
        return 0

    return run_headless(lander, seed, args.max_time, args.verbose)


if __name__ == "__main__":
    raise SystemExit(main())
