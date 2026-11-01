"""Runtime methods shared by :class:`rust_actr.Model`.

The methods mirror the subset of ``actr7.x/tutorial/python/actr.py`` the
tutorial experiments use.  They delegate to the in-process command session,
so the same code that drives the RPC server drives a native model.
"""

from __future__ import annotations

import time as _time
from typing import Any, Callable, Optional

from ._values import render_param_key


class RuntimeMixin:
    """Runtime command surface.  The host class supplies ``_eval``."""

    _session: Any
    _trace: str
    _commands: dict[str, Callable[..., Any]]
    _monitors: dict[str, list[str]]
    _current_window: Optional[str]
    _device_installed: bool

    # -- helpers ---------------------------------------------------------
    def _eval(self, name: str, *args: Any) -> list[Any]:
        raise NotImplementedError

    def _run_result(self, name: str, *args: Any) -> list[Any]:
        raise NotImplementedError

    @staticmethod
    def _first(result: list[Any]) -> Any:
        return result[0] if result else None

    def _single(self, name: str, *args: Any) -> Any:
        return self._first(self._eval(name, *args))

    # -- run/lifecycle ---------------------------------------------------
    def _run_cmd(self, name: str, *args: Any) -> str:
        before = len(self._trace)
        self._run_result(name, *args)
        return self._trace[before:]

    def run(self, seconds: float, real_time: bool = False) -> str:
        return self._run_cmd("run", seconds)

    def run_full_time(self, seconds: float, real_time: bool = False) -> str:
        return self._run_cmd("run-full-time", seconds)

    def run_until(self, time: float, real_time: bool = False) -> str:
        return self._run_cmd("run-until-time", time)

    def run_n_events(self, count: int, real_time: bool = False) -> str:
        return self._run_cmd("run-n-events", count)

    def reset(self) -> str:
        return self._run_cmd("reset")

    def reload(self, compile: bool = False) -> str:
        return self._run_cmd("reload")

    def step(self) -> bool:
        self._ensure_session()
        self._dispatch_fired()
        return bool(self._session.step_once())

    @property
    def time(self) -> float:
        return self._session.time() if self._session is not None else 0.0

    @property
    def trace(self) -> str:
        return self._trace

    def take_trace(self) -> str:
        text = self._trace
        self._trace = ""
        return text

    def clear_trace(self) -> None:
        self._trace = ""
        if self._session is not None:
            self._session.take_trace()

    @property
    def compiled(self) -> bool:
        return self._session is not None

    # -- commands and callbacks -----------------------------------------
    def add_command(
        self,
        name: str,
        function: Optional[Callable[..., Any]] = None,
        documentation: str = "No documentation provided.",
        single: bool = True,
        local_name: Optional[str] = None,
        encoded: bool = False,
    ) -> Any:
        key = name.upper()
        if function is not None:
            self._commands[key] = function
        self._eval("add-act-r-command", name)
        return True

    def remove_command(self, name: str) -> Any:
        self._commands.pop(name.upper(), None)
        return self._single("remove-act-r-command", name)

    def monitor_command(self, original: str, monitor: str) -> Any:
        key = original.upper()
        monitors = self._monitors.setdefault(key, [])
        if monitor.upper() not in monitors:
            monitors.append(monitor.upper())
        return True

    def remove_command_monitor(self, original: str, monitor: str) -> Any:
        key = original.upper()
        monitors = self._monitors.get(key, [])
        self._monitors[key] = [m for m in monitors if m != monitor.upper()]
        return True

    def call_command(self, command: str, *parameters: Any) -> Any:
        fn = self._commands.get(command.upper())
        if fn is None:
            return self._single("call-command", command, *parameters)
        return fn(*parameters)

    def get_time(self, model_time: bool = True) -> Any:
        if model_time:
            return int(round(self.time * 1000.0))
        return int(round(_time.monotonic() * 1000.0))

    def random(self, value: Any) -> Any:
        return self._single("act-r-random", value)

    def permute_list(self, items: list[Any]) -> list[Any]:
        return self._first(self._eval("permute-list", items))

    def model_output(self, text: str) -> Any:
        return self._single("model-output", text)

    def print_warning(self, text: str) -> Any:
        return self._single("print-warning", text)

    def correlation(self, results: list[Any], data: list[Any], output: bool = True) -> Any:
        return self._single("correlation", results, data, output)

    def mean_deviation(self, results: list[Any], data: list[Any], output: bool = True) -> Any:
        return self._single("mean-deviation", results, data, output)

    def no_output(self, command: str, *params: Any) -> Any:
        return self._first(self._eval("no-output", command, *params))

    # -- declarative memory and buffers ---------------------------------
    def buffer_read(self, buffer: str) -> Any:
        value = self._single("buffer-read", buffer)
        # A direct `set-buffer-chunk` spec is unnamed in the engine; the task
        # client still needs a handle for `chunk-slot-value`.
        return buffer if value is None else value

    def buffer_slot_value(self, buffer: str, slot: str) -> Any:
        return self._single("buffer-slot-value", buffer, slot)

    def chunk_slot_value(self, chunk: str, slot: str) -> Any:
        value = self._single("chunk-slot-value", chunk, slot)
        if value is None:
            value = self._single("buffer-slot-value", chunk, slot)
        value = _unquote(value)
        # The command layer sends slot values as printed text; tutorial code
        # does arithmetic on numeric slots.
        if isinstance(value, str):
            try:
                number = float(value)
            except ValueError:
                return value
            return int(number) if number.is_integer() else number
        return value

    def set_buffer_chunk(
        self,
        buffer: str,
        chunk: Any,
        requested: bool = True,
    ) -> Any:
        if isinstance(chunk, (list, tuple)):
            spec = list(chunk)
            if spec and not any(str(item).lower() == "isa" for item in spec):
                declared = getattr(self, "_buffer_types", {}).get(buffer.upper())
                if declared:
                    spec = ["isa", declared, *spec]
        else:
            spec = chunk
        return self._single("set-buffer-chunk", buffer, spec, requested)

    def clear_buffer(self, buffer: str) -> Any:
        return self._single("clear-buffer", buffer)

    def mod_focus(self, *mods: Any) -> Any:
        return self._single("mod-focus", list(mods))

    def mod_chunk(self, chunk: str, *mods: Any) -> Any:
        return self._single("mod-chunk", chunk, list(mods))

    def buffer_status(self, *buffers: str) -> Any:
        return self._eval("buffer-status", *buffers)

    def buffer_chunk(self, *buffers: str) -> Any:
        result = self._eval("buffer-chunk", *buffers)
        return result[0] if len(result) == 1 else result

    def printed_buffer_chunk(self, *buffers: str) -> Any:
        result = self._eval("printed-buffer-chunk", *buffers)
        return result[0] if len(result) == 1 else result

    def chunk_p(self, name: str) -> Any:
        return self._single("chunk-p", name)

    def copy_chunk(self, name: str) -> Any:
        return self._single("copy-chunk", name)

    def set_chunk_slot_value(self, chunk: str, slot: str, value: Any) -> Any:
        return self._single("set-chunk-slot-value", chunk, slot, value)

    def extend_possible_slots(self, slot: str, warn: bool = True) -> Any:
        return self._single("extend-possible-slots", slot)

    # -- parameters ------------------------------------------------------
    def set_parameter_value(self, key: str, value: Any) -> Any:
        return self._single("set-parameter-value", render_param_key(key), value)

    def get_parameter_value(self, key: str) -> Any:
        return self._single("get-parameter-value", render_param_key(key))

    def set_system_parameter_value(self, key: str, value: Any) -> Any:
        return self._single("set-system-parameter-value", render_param_key(key), value)

    def get_system_parameter_value(self, key: str) -> Any:
        return self._single("get-system-parameter-value", render_param_key(key))

    def sdp(self, *params: Any) -> Any:
        return self._eval("sdp", *params)

    def pdisable(self, *names: str) -> Any:
        return self._eval("pdisable", *names)

    def penable(self, *names: str) -> Any:
        return self._eval("penable", *names)

    def trigger_reward(self, value: Any, maintenance: bool = False) -> Any:
        return self._single("trigger-reward", value)

    def set_similarities(self, *pairs: Any) -> Any:
        entries = _entries(pairs)
        flat: list[Any] = []
        for entry in entries:
            flat.extend(entry)
        return self._single("set-similarities", flat)

    def set_base_levels(self, *pairs: Any) -> Any:
        entries = _entries(pairs)
        flat: list[Any] = []
        for entry in entries:
            flat.extend(entry)
        return self._single("set-base-levels", flat)

    def set_all_base_levels(self, level: Any, creation: Optional[Any] = None) -> Any:
        if creation is None:
            return self._single("set-all-base-levels", level)
        return self._single("set-all-base-levels", level, creation)

    # -- queries ---------------------------------------------------------
    def whynot(self, *names: str) -> Any:
        return self._eval("whynot", *names)

    def whynot_dm(self, *names: str) -> Any:
        return self._eval("whynot-dm", *names)

    def sdm(self, *spec: Any) -> Any:
        return self._eval("sdm", *spec)

    def dm(self, *spec: Any) -> Any:
        return self._eval("dm", *spec)

    def pp(self, *names: str) -> Any:
        return self._eval("pp", *names)

    def all_productions(self) -> Any:
        return self._first(self._eval("all-productions"))

    def print_visicon(self) -> Any:
        return self._single("print-visicon")

    def printed_visicon(self, show_pending: bool = False) -> Any:
        return self._single("printed-visicon", show_pending)

    def print_audicon(self) -> Any:
        return self._single("print-audicon")

    def simulate_retrieval_request(self, *spec: Any) -> Any:
        return self._eval("simulate-retrieval-request", *spec)

    def record_history(self, *params: Any) -> Any:
        return self._eval("record-history", *params)

    def stop_recording_history(self, *params: Any) -> Any:
        return self._eval("stop-recording-history", *params)

    def predict_bold_response(self, *params: Any) -> Any:
        return self._single("predict-bold-response", *params)

    def add_visicon_features(self, *features: Any) -> Any:
        return self._single("add-visicon-features", *features)

    def modify_visicon_features(self, *features: Any) -> Any:
        return self._single("modify-visicon-features", *features)

    def delete_visicon_features(self, *features: Any) -> Any:
        return self._single("delete-visicon-features", *features)

    def delete_all_visicon_features(self) -> Any:
        return self._single("delete-all-visicon-features")

    # -- windows and devices --------------------------------------------
    def open_exp_window(
        self,
        title: str,
        visible: bool = True,
        width: int = 300,
        height: int = 300,
        x: int = 300,
        y: int = 300,
    ) -> Any:
        result = self._single(
            "open-exp-window",
            title,
            _pairs(
                visible=visible, width=width, height=height, x=x, y=y
            ),
        )
        self._current_window = result if isinstance(result, str) else title
        return result

    def clear_exp_window(self, window: Optional[str] = None) -> Any:
        return self._single("clear-exp-window", window or self._current_window)

    def add_text_to_exp_window(
        self,
        window: Optional[str],
        text: str,
        x: int = 0,
        y: int = 0,
        color: str = "black",
        height: int = 20,
        width: int = 75,
        font_size: int = 12,
    ) -> Any:
        return self._single(
            "add-text-to-exp-window",
            window or self._current_window,
            text,
            _pairs(
                x=x,
                y=y,
                color=color,
                height=height,
                width=width,
                **{"font-size": font_size},
            ),
        )

    def add_line_to_exp_window(
        self,
        window: Optional[str],
        start: Any,
        end: Any,
        color: str = "black",
    ) -> Any:
        return self._single(
            "add-line-to-exp-window", window or self._current_window, start, end, color
        )

    def add_button_to_exp_window(
        self,
        window: Optional[str],
        text: str = "",
        x: int = 0,
        y: int = 0,
        action: Any = None,
        height: int = 20,
        width: int = 75,
        color: str = "gray",
    ) -> Any:
        options = _pairs(
            text=text, x=x, y=y, height=height, width=width, color=color
        )
        if action is not None:
            options.append(["action", action])
        return self._single(
            "add-button-to-exp-window", window or self._current_window, options
        )

    def remove_items_from_exp_window(self, window: Optional[str], *items: Any) -> Any:
        return self._single(
            "remove-items-from-exp-window", window or self._current_window, *items
        )

    def modify_line_for_exp_window(
        self,
        line: str,
        start: Any,
        end: Any,
        color: Any = False,
    ) -> Any:
        return self._single(
            "modify-line-for-exp-window", line, start, end, color or None
        )

    def modify_text_for_exp_window(
        self,
        item: str,
        x: Optional[int] = None,
        y: Optional[int] = None,
    ) -> Any:
        options: list[list[Any]] = []
        if x is not None:
            options.append(["x", x])
        if y is not None:
            options.append(["y", y])
        return self._single("modify-text-for-exp-window", item, options)

    def install_device(self, window: Any = None) -> Any:
        result = self._single("install-device", window or self._current_window)
        self._device_installed = True
        return result

    def start_hand_at_mouse(self) -> Any:
        return self._single("start-hand-at-mouse")

    def process_events(self) -> None:
        return None

    def visible_virtuals_available(self) -> Any:
        return self._single("visible-virtuals-available?")

    def get_articulation_time(self, text: str, time_in_ms: bool = False) -> Any:
        return self._single("get-articulation-time", text, time_in_ms)

    def register_articulation_time(
        self, text: str, time: Any, time_in_ms: bool = False
    ) -> Any:
        return self._single("register-articulation-time", text, time, time_in_ms)

    def get_syllable_count(self, word: str) -> Any:
        return self._single("get-syllable-count", word)

    def register_syllable_count(self, word: str, syllables: int) -> Any:
        return self._single("register-syllable-count", word, syllables)

    def remove_syllable_count(self, word: str) -> Any:
        return self._single("remove-syllable-count", word)

    def new_tone_sound(
        self,
        freq: Any,
        duration: Any,
        onset: Any = False,
        time_in_ms: bool = False,
    ) -> Any:
        return self._single("new-tone-sound", freq, duration, onset, time_in_ms)

    def new_digit_sound(
        self,
        digit: Any,
        onset: Any = False,
        time_in_ms: bool = False,
    ) -> Any:
        return self._single("new-digit-sound", digit, onset, time_in_ms)

    def new_word_sound(
        self,
        word: Any,
        onset: Any = False,
        location: str = "external",
        time_in_ms: bool = False,
    ) -> Any:
        return self._single("new-word-sound", word, onset, location, time_in_ms)

    def schedule_event_relative(
        self,
        time_delay: Any,
        action: Any,
        params: Any = None,
        module: str = ":NONE",
        priority: Any = 0,
        maintenance: bool = False,
        destination: Any = None,
        details: Any = None,
        output: bool = True,
        time_in_ms: bool = False,
        precondition: Any = None,
    ) -> Any:
        delay = time_delay / 1000.0 if time_in_ms else time_delay
        return self._single(
            "schedule-event-relative", delay, action, _event_options(params, output)
        )

    def schedule_event(
        self,
        time: Any,
        action: Any,
        params: Any = None,
        module: str = ":NONE",
        priority: Any = 0,
        maintenance: bool = False,
        destination: Any = None,
        details: Any = None,
        output: bool = True,
        time_in_ms: bool = False,
        precondition: Any = None,
    ) -> Any:
        when = time / 1000.0 if time_in_ms else time
        return self._single("schedule-event", when, action, _event_options(params, output))

    def schedule_event_now(
        self,
        action: Any,
        params: Any = None,
        module: str = ":NONE",
        priority: Any = 0,
        maintenance: bool = False,
        destination: Any = None,
        details: Any = None,
        output: bool = True,
        precondition: Any = None,
    ) -> Any:
        return self._single("schedule-event-now", action, _event_options(params, output))


def _pairs(**options: Any) -> list[list[Any]]:
    """The Python client's window-option shape: `[[key, value], ...]`."""
    return [[key, value] for key, value in options.items()]


def _unquote(value: Any) -> Any:
    if isinstance(value, str) and len(value) >= 2 and value[0] == value[-1]:
        if value[0] in ("'", '"'):
            return value[1:-1]
    return value


def _event_options(params: Any, output: bool) -> dict[str, Any]:
    options: dict[str, Any] = {"params": list(params or []), "output": output}
    return options


def _entries(pairs: Any) -> list[Any]:
    if not pairs:
        return []
    if len(pairs) == 1 and isinstance(pairs[0], (list, tuple)):
        candidate = pairs[0]
        if candidate and all(isinstance(item, (list, tuple)) for item in candidate):
            return list(candidate)
        return [candidate]
    return list(pairs)
