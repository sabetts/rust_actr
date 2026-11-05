//! Event dispatcher (`exec`).
use super::compilation::buffer_module;
use super::motor::{key_is_punch, key_xy};
use super::perception::{audio_event_chunk, sound_chunk};
use super::*;
impl Engine {
    fn scheduled_command_known(&self, name: &str) -> bool {
        let key = name.to_uppercase();
        matches!(
            key.as_str(),
            "SET-IMAGINAL-FREE" | "SET-IMAGINAL-ERROR" | "CLEAR-EXP-WINDOW"
        ) || self.task_commands.contains(&key)
            || self.user_fns.contains_key(&key)
    }

    pub(super) fn exec(&mut self, event: Event, show: bool) -> Result<(), EngineError> {
        match event {
            Event::SetBuffer {
                buffer,
                chunk,
                requested,
                shown,
            } => {
                let mut inst = self.instantiate(&buffer, chunk)?;
                // `set-buffer-chunk` on a buffer with `reuse?` nil (a
                // `buffer-requires-copies` buffer) converts the spec with
                // `define-chunks-fct` and so creates a fresh chunk auto-named
                // `CHUNK{k}` (`buffers.lisp:1360`); the trace still prints the
                // spec/chunk-def, so capture it before naming.
                let text = shown.unwrap_or_else(|| inst.show());
                if inst.name.is_none() {
                    if self.model.no_reuse_buffers.iter().any(|b| b == &buffer) {
                        // `buffer-requires-copies`: each set makes a fresh
                        // chunk auto-named `CHUNK{k}` (`buffers.lisp:1360`).
                        let n = *self.name_count.entry("CHUNK".to_string()).or_insert(0);
                        self.name_count.insert("CHUNK".to_string(), n + 1);
                        inst.name = Some(format!("CHUNK{n}"));
                    } else {
                        // A reusable buffer names its chunk once and reuses it
                        // (`act-r-buffer-reuse-chunk`, `buffers.lisp:1351`):
                        // `{BUFFER}-CHUNK{n}` via `new-name-fct`.
                        let base = format!("{buffer}-CHUNK");
                        let name = match self.buffer_reuse.get(&buffer).cloned() {
                            Some(n) => n,
                            None => {
                                let n = *self.name_count.entry(base.clone()).or_insert(0);
                                self.name_count.insert(base.clone(), n + 1);
                                let name = format!("{base}{n}");
                                self.buffer_reuse.insert(buffer.clone(), name.clone());
                                name
                            }
                        };
                        inst.name = Some(name);
                    }
                }
                if buffer == "IMAGINAL"
                    && !self.model.no_reuse_buffers.iter().any(|b| b == "IMAGINAL")
                {
                    // The imaginal buffer's reusable chunk is `IMAGINAL-CHUNK0`;
                    // `set-buffer-chunk` copies the new content into it and
                    // `buffer-read` returns that name, even though the trace
                    // prints the action's copy name.
                    inst.name = Some(self.reuse_buffer_name("IMAGINAL"));
                }
                if buffer == "AURAL" || buffer == "AURAL-LOCATION" {
                    if let Some(source) = inst.name.clone() {
                        let n = *self.copy_count.entry(source.clone()).or_insert(0);
                        self.copy_count.insert(source.clone(), n + 1);
                        let copy = format!("{source}-{n}");
                        self.audio_copied_from.insert(copy.clone(), source);
                        inst.name = Some(copy);
                    }
                }
                if show {
                    let mut details = format!("SET-BUFFER-CHUNK {buffer} {text}");
                    if !requested {
                        details.push_str(" NIL");
                    }
                    // The SET event prints under the buffer's owning
                    // module (the `:module` of `schedule-set-buffer-chunk`).
                    let module = match buffer.as_str() {
                        "RETRIEVAL" => "DECLARATIVE",
                        "VISUAL-LOCATION" | "VISUAL" => "VISION",
                        "MANUAL" => "MOTOR",
                        "AURAL" | "AURAL-LOCATION" => "AUDIO",
                        other => other,
                    };
                    self.emit(module, details);
                }
                if buffer == "GOAL"
                    && self.cfg.threads
                    && !self.goal_set.iter().any(|c| c.name == inst.name)
                {
                    self.goal_set.push(inst.clone());
                }
                self.buffer_mut(&buffer).chunk = Some(inst);
                self.buffer_mut(&buffer).failed = false;
                self.buffer_mut(&buffer).modified = false;
                self.buffer_mut(&buffer).requested = requested;
                if buffer == "GOAL" {
                    self.bold_point("GOAL");
                    self.sperling_capture_row();
                }
                self.changed();
            }
            Event::OverwriteBuffer {
                buffer,
                chunk,
                requested,
                shown,
                module,
                copy_name,
            } => {
                let mut inst = self.instantiate(&buffer, chunk)?;
                let source = shown.clone().unwrap_or_else(|| inst.show());
                if buffer == "VISUAL" {
                    // The VISUAL buffer holds a `VISUAL-CHUNK0` copy of the
                    // overwritten chunk (`chunk-copied-from-fct`), so
                    // `buffer-chunk` prints `VISUAL-CHUNK0 [SOURCE]`.
                    self.vl_copied_from
                        .insert("VISUAL-CHUNK0".to_string(), source.clone());
                    inst.name = Some("VISUAL-CHUNK0".to_string());
                } else if buffer == "VISUAL-LOCATION" {
                    if let Some(copy) = copy_name {
                        // Auto-overstuff already made the `{base}-{k}` copy.
                        inst.name = Some(copy.clone());
                        self.vl_copied_from.insert(copy, source.clone());
                    } else {
                        // `@visual-location> vl-1` copies into `VL-1-{k}`.
                        let n = *self.copy_count.entry(source.clone()).or_insert(0);
                        self.copy_count.insert(source.clone(), n + 1);
                        let copy = format!("{source}-{n}");
                        self.vl_copied_from.insert(copy.clone(), source.clone());
                        inst.name = Some(copy);
                    }
                } else if (buffer == "AURAL" || buffer == "AURAL-LOCATION") && inst.name.is_some() {
                    let base = inst.name.clone().unwrap();
                    let n = *self.copy_count.entry(base.clone()).or_insert(0);
                    self.copy_count.insert(base.clone(), n + 1);
                    let copy = format!("{base}-{n}");
                    self.audio_copied_from.insert(copy.clone(), base);
                    inst.name = Some(copy);
                }
                if show {
                    let mut details = format!("OVERWRITE-BUFFER-CHUNK {buffer} {source}");
                    if !requested {
                        details.push_str(" NIL");
                    }
                    self.emit(&module, details);
                }
                if buffer == "GOAL"
                    && self.cfg.threads
                    && !self.goal_set.iter().any(|c| c.name == inst.name)
                {
                    self.goal_set.push(inst.clone());
                }
                self.buffer_mut(&buffer).chunk = Some(inst);
                self.buffer_mut(&buffer).failed = false;
                self.buffer_mut(&buffer).modified = false;
                self.buffer_mut(&buffer).requested = requested;
                if buffer == "GOAL" {
                    self.sperling_capture_row();
                }
                self.changed();
            }
            Event::EraseBuffer { buffer, module } => {
                // `erase-buffer` (`buffers.lisp:1273`): drop the chunk but
                // keep the buffer state (no clear-buffer).
                if show {
                    self.emit(&module, format!("ERASE-BUFFER {buffer}"));
                }
                self.buffer_mut(&buffer).chunk = None;
                self.buffer_mut(&buffer).failed = false;
                self.buffer_mut(&buffer).modified = false;
                self.changed();
            }
            Event::VocalFree => {
                if self.time_ms >= self.vocal_end_ms {
                    self.vocal_busy = false;
                    self.vocal_end_ms = 0;
                }
                self.changed();
            }
            Event::Speak { style, text } => {
                self.emit("SPEECH", format!("{} TEXT {text}", style.trace()));
                self.vocal_busy = true;
                self.vocal_prep_busy = true;
                // `compute-prep-time` (`general-pm.lisp:670`): a fresh module
                // or a different style name preps all `num-to-prepare`
                // features (3, `speech.lisp:278`); the same style preps only
                // `feat-differences` (0 for the same text, else 2).
                let feats = match &self.last_speech {
                    None => 3,
                    Some((prev, _)) if *prev != style => 3,
                    Some((_, prev)) if prev.eq_ignore_ascii_case(&text) => 0,
                    Some(_) => 2,
                };
                self.last_speech = Some((style, text.clone()));
                let prep = feats * self.cfg.feat_prep_ms;
                self.push(
                    self.time_ms + prep,
                    Prio::Num(0),
                    Out::Always,
                    Event::SpeechPrep { style, text },
                );
            }
            Event::SpeechPrep { style, text } => {
                // `perform-movement`: preparation is complete, but articulation
                // keeps the vocal buffer busy (`?vocal> preparation` vs
                // `?vocal> state`).
                self.vocal_prep_busy = false;
                // `perform-movement`: initiation-complete at init-time, then
                // finish at exec + articulation (`speech.lisp:281`/`285`).
                // The hear-self sound onset equals the exec time.
                let init = self.cfg.init_ms;
                let art = self.articulation_time_ms(&text);
                // `maybe-execute-movement`: execution waits until the current
                // utterance finishes, while preparation has already run.
                let exec_start = self.time_ms.max(self.vocal_end_ms);
                let init_at = exec_start + init;
                let finish_at = init_at + art;
                self.vocal_end_ms = self.vocal_end_ms.max(finish_at);
                // `queue-output-events` (`speech.lisp:294`/`:314`) creates the
                // sound -- when `:hear-self` admits this style -- *before*
                // scheduling the device notification, so `new-sound` precedes
                // the `SPEAK <text>` line at the same instant.
                if self.cfg.hear_self.hears(style) {
                    let evt = self.audio_events;
                    self.audio_events += 1;
                    self.push(
                        init_at,
                        Prio::Num(0),
                        Out::Medium,
                        Event::NewSound {
                            evt,
                            offset: finish_at,
                        },
                    );
                }
                self.push(
                    init_at,
                    Prio::Num(0),
                    Out::Always,
                    Event::SpeechInit {
                        style,
                        text: text.clone(),
                    },
                );
                self.push(finish_at, Prio::Num(0), Out::Never, Event::VocalFree);
            }
            Event::SpeechInit { style, text } => {
                // `notify-speech-device` (`speech.lisp:267`): with no speech
                // device installed, a `speak` start schedules
                // `default-speech-action` at `:output medium`, whose details
                // render as `SPEAK <text>`. With a device the module notifies
                // it instead (no trace), and `subvocalize` only notifies when
                // `:notify-subvocalize` is on (off by default).
                if style == SpeechStyle::Speak {
                    if self.speech_device {
                        self.fired_commands.push((
                            "output-speech".to_string(),
                            self.model.name.clone(),
                            text.clone(),
                        ));
                    } else if self.cfg.detail.shows(Out::Medium) {
                        self.emit_at(Out::Medium, "SPEECH", format!("{} {text}", style.trace()));
                    }
                }
            }
            Event::NewSound { evt, offset } => {
                if show {
                    self.emit("AUDIO", "new-sound".to_string());
                }
                self.record_audicon_snapshot();
                // `stuff-sound-buffer` only sets AURAL-LOCATION while the
                // buffer is empty (`audio.lisp:716`); later sounds leave the
                // first location in place. Aural detection/recode delay.
                if self.aural_loc_evt.is_none() {
                    let excludes_self = self.model.audloc_neg.iter().any(|(slot, value)| {
                        slot == "LOCATION"
                            && matches!(value, Sexp::Sym(s) if s.eq_ignore_ascii_case("SELF"))
                    });
                    let event = if excludes_self {
                        Event::Emit {
                            module: "AUDIO".to_string(),
                            line: "find-sound-failure".to_string(),
                        }
                    } else {
                        Event::AuralLocation { evt }
                    };
                    self.push(self.time_ms + 300, Prio::Num(0), Out::Medium, event);
                }
                // `find-sound` (`audio.lisp:1045`): a word/tone event whose
                // chunk has no `offset`/`duration` schedules
                // `audio-event-ended` at the sound's offset.
                self.push_m(
                    offset,
                    Prio::Num(0),
                    Out::Always,
                    false,
                    Event::AudioEventEnded { evt },
                );
            }
            Event::AuralLocation { evt } => {
                self.aural_loc_evt = Some(evt);
                self.emit(
                    "AUDIO",
                    format!("SET-BUFFER-CHUNK AURAL-LOCATION AUDIO-EVENT{evt} NIL"),
                );
            }
            Event::AudioEventEnded { evt } => {
                // `audio-event-ended` (`audio.lisp:1077`) fires only while the
                // `AURAL-LOCATION` buffer still holds this event
                // (`aural-location-update-check`), then `mod-buffer-chunk`s
                // its `offset`/`duration`. The scheduled event's own trace
                // line (`AUDIO-EVENT-ENDED <ename>`) precedes the mod.
                if self.aural_loc_evt == Some(evt) {
                    let ename = format!("AUDIO-EVENT{evt}");
                    if let Some(idx) = self.sounds.iter().position(|s| s.ename == ename) {
                        self.sync_audio_chunks(idx);
                        self.update_audio_location_buffer(idx);
                    }
                    // The scheduled event's own line is `:output high`
                    // (`audio.lisp:1048`), so only a high-detail run prints it:
                    // of the refs that reach `audio-event-ended`, only
                    // `unstuff-aural` shows it �?" its model is the one that sets
                    // `(sgp :trace-detail high)`. The `mod-buffer-chunk` that
                    // follows is `medium` and appears in all of them.
                    if self.cfg.detail.shows(Out::Always) {
                        self.emit_at(
                            Out::Always,
                            "AUDIO",
                            format!("AUDIO-EVENT-ENDED AUDIO-EVENT{evt}"),
                        );
                    }
                    self.emit("AUDIO", "MOD-BUFFER-CHUNK AURAL-LOCATION".to_string());
                    self.changed();
                }
            }
            Event::EndOngoingSound { id } => {
                self.end_ongoing_sound(id)?;
            }
            Event::ToneSound { idx } => {
                // `new-sound-event :around` (`audio.lisp:733`): `new-sound`
                // at the onset (`:output medium`), then `stuff-sound-buffer`
                // at `detect-at-time` = `onset + tone-detect-delay` as a
                // maintenance event.
                if show {
                    self.emit("AUDIO", "new-sound".to_string());
                }
                self.record_audicon_snapshot();
                let s = self.sounds[idx].clone();
                self.push_m(
                    s.onset + s.delay,
                    Prio::Num(0),
                    Out::Always,
                    true,
                    Event::StuffSound { idx },
                );
            }
            Event::StuffSound { idx } => {
                // `stuff-sound-buffer` -> `find-sound … :stuffed t`
                // (`audio.lisp:716`): set AURAL-LOCATION to the sound's
                // `audio-event` chunk with `:requested nil`.
                //
                // The overwrite test wraps the `find-sound` call, so a sound
                // that may not stuff also must not consume that call's RNG
                // draw — check before drawing.
                if !self.can_stuff_aural() {
                    return Ok(());
                }
                self.sync_audio_chunks(idx);
                let s = self.sounds[idx].clone();
                // `find-sound` (`audio.lisp:1031`) picks
                // `(random-item matching-chunks)`; even a single live tone
                // consumes one draw, and it lands between the visual finds.
                let _ = self.rng_draw(1);
                let chunk = audio_event_chunk(&s, self.time_ms, self.cfg.audio_ms_times);
                // `find-sound` with `:stuffed t` and a chunk already in the
                // buffer *overwrites* it (`schedule-overwrite-buffer-chunk`,
                // `audio.lisp:1039`); an empty buffer gets a plain set. The two
                // print different lines, and `unstuff-aural`'s later runs are
                // all overwrites.
                let overwrite = self
                    .buffers
                    .get("AURAL-LOCATION")
                    .is_some_and(|b| b.chunk.is_some());
                if overwrite {
                    self.now(
                        Prio::Num(10),
                        Out::Low,
                        Event::OverwriteBuffer {
                            buffer: "AURAL-LOCATION".to_string(),
                            chunk: NewChunk::Instance(chunk),
                            requested: false,
                            shown: None,
                            module: "AUDIO".to_string(),
                            copy_name: None,
                        },
                    );
                } else {
                    self.now(
                        Prio::Num(10),
                        Out::Low,
                        Event::SetBuffer {
                            buffer: "AURAL-LOCATION".to_string(),
                            chunk: NewChunk::Instance(chunk),
                            requested: false,
                            shown: None,
                        },
                    );
                }
                // `stuff-sound-buffer` also arms the unstuff
                // (`audio.lisp:1056`): `t` unstuffs at `offset + decay-time`, a
                // number that many ms from now, with the same "still the
                // unchanged auto-stuffed copy" precondition.
                // `stuff-sound-buffer` also arms the unstuff
                // (`audio.lisp:1056`): `t` unstuffs at `offset + decay-time`, a
                // number that many ms from now. Each stuff arms its own — the
                // probe shows all four accumulating, and the stale ones failing
                // their precondition.
                if let Some(delay) = self.cfg.unstuff_aural {
                    let at = match delay {
                        Some(ms) => Some(self.time_ms + ms),
                        None => s.offset.map(|offset| offset + self.cfg.decay_time_ms),
                    };
                    if let Some(at) = at {
                        self.push_m(
                            at,
                            Prio::Min,
                            Out::Never,
                            true,
                            Event::UnstuffAural {
                                copy: s.ename.clone(),
                            },
                        );
                    }
                }
                // `find-sound` also schedules the end check at the sound's
                // `offset` (`audio.lisp:1048`). Only the speech path reached it
                // before, so a tone never reported `audio-event-ended`.
                // `audio-event-ended` is scheduled `:output high`
                // (`audio.lisp:1048`): only a high-detail run reaches it. Of
                // the tone refs, `unstuff-aural` is the one whose model sets
                // `(sgp :trace-detail high)`; `sperling` (medium) shows neither
                // this event nor its mod, and `hear-self`'s speech sound gets
                // its mod from the `new-sound` path instead.
                if let Ok(evt) = s.ename.trim_start_matches("AUDIO-EVENT").parse::<u64>() {
                    // The buffer now holds this event, which is what the end
                    // check's precondition tests (`aural-location-update-check`,
                    // `audio.lisp:1068`).
                    self.aural_loc_evt = Some(evt);
                    if self.cfg.detail.shows(Out::Always) {
                        if let Some(offset) = s.offset {
                            self.push_m(
                                offset,
                                Prio::Num(0),
                                Out::Always,
                                false,
                                Event::AudioEventEnded { evt },
                            );
                        }
                    }
                }
            }
            Event::FindSound { spec } => {
                if show {
                    self.emit("AUDIO", "FIND-SOUND".to_string());
                }
                let NewChunk::Spec {
                    slots, neg, rels, ..
                } = spec
                else {
                    return Ok(());
                };
                let mut direct = Vec::new();
                let mut relative = Vec::new();
                for (slot, value) in slots {
                    match value {
                        Value::Sym(s) if s == "HIGHEST" || s == "LOWEST" => {
                            relative.push((slot, s == "HIGHEST"));
                        }
                        other => direct.push((slot, other)),
                    }
                }
                let now = self.time_ms;
                let mut candidates: Vec<(usize, ChunkInst)> = self
                    .sounds
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| {
                        s.onset <= now
                            && now >= s.onset + s.delay
                            && s.offset.map_or(true, |o| o + self.cfg.decay_time_ms >= now)
                    })
                    .map(|(idx, s)| (idx, audio_event_chunk(s, now, self.cfg.audio_ms_times)))
                    .filter(|(_, c)| self.chunk_matches_req(c, &direct, &neg, &rels))
                    .collect();
                for (slot, highest) in relative {
                    let mut best: Option<Value> = None;
                    for (_, c) in &candidates {
                        let value = c.get(&slot);
                        let number = match value {
                            Value::Int(i) => Some(i as f64),
                            Value::Num(x) => Some(x),
                            _ => None,
                        };
                        let Some(number) = number else {
                            continue;
                        };
                        let better = match best.as_ref().and_then(|v| match v {
                            Value::Int(i) => Some(*i as f64),
                            Value::Num(x) => Some(*x),
                            _ => None,
                        }) {
                            None => true,
                            Some(old) => (highest && number > old) || (!highest && number < old),
                        };
                        if better {
                            best = Some(Value::Num(number));
                        }
                    }
                    if let Some(best) = best {
                        candidates.retain(|(_, c)| self.value_eq(&c.get(&slot), &best));
                    }
                }
                if candidates.is_empty() {
                    return Ok(());
                }
                let pick = self.rng_draw(candidates.len());
                let (idx, chunk) = candidates.swap_remove(pick);
                let event = chunk
                    .name
                    .as_deref()
                    .and_then(|n| n.trim_start_matches("AUDIO-EVENT").parse::<u64>().ok());
                if let Some(evt) = event {
                    self.aural_loc_evt = Some(evt);
                }
                self.sync_audio_chunks(idx);
                self.now(
                    Prio::Num(10),
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "AURAL-LOCATION".to_string(),
                        chunk: NewChunk::Instance(chunk),
                        requested: true,
                        shown: None,
                    },
                );
            }
            Event::AttendSound { idx, display } => {
                self.sounds[idx].attended = SoundAttended::Requested;
                let s = self.sounds[idx].clone();
                if let Some(buf) = self.buffers.get_mut("AURAL") {
                    buf.busy = true;
                }
                if self.cfg.detail.shows(Out::Medium) {
                    self.emit_at(Out::Medium, "AUDIO", format!("ATTEND-SOUND {display}"));
                }
                self.push(
                    self.time_ms + s.recode,
                    Prio::Num(0),
                    Out::Medium,
                    Event::AudioEncodingComplete { idx },
                );
            }
            Event::AudioEncodingComplete { idx } => {
                // `audio-encoding-complete` (`audio.lisp:1205`): free the
                // module, give the sound chunk `content`/`kind`/`event`, then
                // set the AURAL buffer.
                self.sounds[idx].attended = SoundAttended::Attended;
                self.sync_sound_chunk(idx);
                let s = self.sounds[idx].clone();
                if let Some(buf) = self.buffers.get_mut("AURAL") {
                    buf.busy = false;
                }
                if self.cfg.detail.shows(Out::Always) {
                    self.emit_at(
                        Out::Always,
                        "AUDIO",
                        format!("AUDIO-ENCODING-COMPLETE {}", s.ename),
                    );
                }
                let chunk = sound_chunk(&s);
                self.now(
                    Prio::Num(0),
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "AURAL".to_string(),
                        chunk: NewChunk::Instance(chunk),
                        requested: true,
                        shown: None,
                    },
                );
            }
            Event::ClearGoalThread { name } => {
                if show {
                    self.emit("PROCEDURAL", "CLEAR-BUFFER GOAL".to_string());
                }
                self.goal_set
                    .retain(|g| g.name.as_deref() != Some(name.as_str()));
                if self
                    .buffers
                    .get("GOAL")
                    .and_then(|b| b.chunk.as_ref())
                    .and_then(|c| c.name.as_deref())
                    == Some(name.as_str())
                {
                    self.buffer_mut("GOAL").chunk = None;
                }
                self.buffer_mut("GOAL").failed = false;
                self.changed();
            }
            Event::ClearBuffer { buffer, module } => {
                if show {
                    self.emit(&module, format!("CLEAR-BUFFER {buffer}"));
                }
                let cleared = self.buffer_mut(&buffer).chunk.take();
                self.buffer_mut(&buffer).modified = false;
                if buffer == "AURAL-LOCATION" {
                    // `aural-location-update-check` only fires while the buffer
                    // still holds the event (audio.lisp:1068).
                    self.aural_loc_evt = None;
                }
                if buffer == "GOAL" && self.cfg.threads {
                    if let Some(n) = cleared.as_ref().and_then(|c| c.name.clone()) {
                        self.goal_set
                            .retain(|g| g.name.as_deref() != Some(n.as_str()));
                    }
                }
                self.buffer_mut(&buffer).failed = false;
                // `:notify-on-clear` for declarative: the cleared chunk
                // merges into DM, gaining a reference
                // (`merge-chunk-into-dm`). Empty clears merge nothing.
                if let Some(chunk) = cleared {
                    // `al-check-cleared-chunks` (`associative-learning.lisp:562`):
                    // a source buffer's clear schedules the parameter update at
                    // priority 5 (after the merge, before a retrieval's -2000).
                    let al = if self.cfg.al_eal.is_some() && self.al_is_source_buffer(&buffer) {
                        Some(chunk.name.clone())
                    } else {
                        None
                    };
                    self.merge_into_dm(chunk, &buffer);
                    if let Some(name) = al {
                        let true_name = name
                            .as_ref()
                            .and_then(|n| self.chunk_alias.get(n).cloned())
                            .or(name);
                        if let Some(true_name) = true_name {
                            self.now(
                                Prio::Num(5),
                                Out::Never,
                                Event::AlBufferCleared { chunk: true_name },
                            );
                        }
                    }
                }
                self.changed();
            }
            Event::UnstuffAural { copy } => {
                // `unstuff-buffer` (`audio.lisp:1056`) with `:output nil`: the
                // scheduled event prints nothing; the nested `erase-buffer`
                // line is high-detail (`unstuff-aural` is the only ref that
                // shows it).
                if self.unstuff_aural_valid(&copy) {
                    self.now(
                        Prio::Min,
                        Out::Always,
                        Event::EraseBuffer {
                            buffer: "AURAL-LOCATION".to_string(),
                            module: "AUDIO".to_string(),
                        },
                    );
                }
            }
            Event::UnstuffVl { copy } => {
                // `check-unstuff-buffer` (`vision.lisp:1944`): only an
                // unchanged, unrequested auto-stuffed copy is erased.
                let valid = self.unstuff_valid(&copy);
                if valid {
                    if show {
                        self.emit("VISION", "ERASE-BUFFER VISUAL-LOCATION".to_string());
                    }
                    // Unstuffing does NOT clear the feature's finst
                    // (`remove-finst` is never called on unstuff); the finst
                    // cap/span is what re-exposes it (Sperling).
                    self.buffer_mut("VISUAL-LOCATION").chunk = None;
                    self.buffer_mut("VISUAL-LOCATION").failed = false;
                    self.changed();
                }
            }
            Event::AlBufferCleared { chunk } => {
                // `al-buffer-cleared` (`associative-learning.lisp:667`): update
                // with the chunk's own sources plus the other source buffers.
                self.event_silent = true;
                let mut js = self.al_chunk_sources(&chunk);
                js.extend(self.al_source_buffer_chunks());
                self.al_update(&chunk, &js);
            }
            Event::ModuleModRequest { buffer, slots } => {
                if show {
                    self.emit("PROCEDURAL", format!("MODULE-MOD-REQUEST {buffer}"));
                } else {
                    self.capture_line("PROCEDURAL", &format!("MODULE-MOD-REQUEST {buffer}"));
                }
                if buffer == "IMAGINAL" {
                    self.imag_busy = true;
                    let delay = if self.cfg.vidt {
                        self.randomize_ms(self.cfg.imaginal_delay_ms)
                    } else {
                        self.cfg.imaginal_delay_ms
                    };
                    self.push(
                        self.time_ms + delay,
                        Prio::Num(0),
                        Out::Low,
                        Event::ModBuffer {
                            buffer: buffer.clone(),
                            slots,
                            module: buffer_module(&buffer),
                        },
                    );
                } else {
                    // A goal `*goal>` lands as a low-output mod (visible even
                    // with `:trace-detail low`); other modules keep medium.
                    let out = if buffer == "GOAL" {
                        Out::Low
                    } else {
                        Out::Medium
                    };
                    self.now(
                        Prio::Num(40),
                        out,
                        Event::ModBuffer {
                            buffer: buffer.clone(),
                            slots,
                            module: buffer_module(&buffer),
                        },
                    );
                }
            }
            Event::ModBuffer {
                buffer,
                slots,
                module,
            } => {
                if show {
                    self.emit(&module, format!("MOD-BUFFER-CHUNK {buffer}"));
                } else {
                    self.capture_line(&module, &format!("MOD-BUFFER-CHUNK {buffer}"));
                }
                {
                    let b = self.buffer_mut(&buffer);
                    let chunk = b
                        .chunk
                        .as_mut()
                        .ok_or_else(|| bad(format!("MOD-BUFFER-CHUNK with empty {buffer}")))?;
                    for (s, v) in slots.iter() {
                        match chunk.slots.iter_mut().find(|(k, _)| k == s) {
                            Some((_, old)) => *old = v.clone(),
                            None => chunk.slots.push((s.clone(), v.clone())),
                        }
                    }
                }
                // `check-unstuff-buffer` sees a modified (non-copy) chunk.
                self.buffer_mut(&buffer).modified = true;
                // A `*imaginal>` mod-request frees the imaginal module once
                // its delayed change lands (`module-mod-request`).
                if buffer == "IMAGINAL" && module == "IMAGINAL" {
                    self.imag_busy = false;
                }
                if self.cfg.tracker {
                    let names: Vec<String> = slots.iter().map(|(s, _)| s.clone()).collect();
                    for s in names {
                        self.tracker_check_buffer_changes(&buffer, Some(&s));
                    }
                }
                if buffer == "GOAL" {
                    self.sperling_capture_row();
                }
                self.changed();
            }
            Event::ModuleRequest {
                buffer,
                spec,
                var_slots,
                trace,
            } => {
                if trace {
                    if show {
                        self.emit("PROCEDURAL", format!("MODULE-REQUEST {buffer}"));
                    } else {
                        self.capture_line("PROCEDURAL", &format!("MODULE-REQUEST {buffer}"));
                    }
                }
                if !trace {
                    // A task `module-request` dispatches straight to the
                    // module (`request-module`, `modules.lisp:819`); it is not
                    // a buffer change, so it must not wake conflict resolution
                    // (the module's own set/clear does that).
                    self.event_silent = true;
                }
                match buffer.as_str() {
                    "RETRIEVAL" => {
                        // dm-request: abort any in-flight retrieval, then start.
                        self.dm_gen += 1;
                        self.dm_busy = true;
                        self.buffers.get_mut("RETRIEVAL").unwrap().busy = true;
                        self.buffers.get_mut("RETRIEVAL").unwrap().failed = false;
                        self.now(
                            Prio::Num(-2000),
                            Out::Medium,
                            Event::StartRetrieval { spec, var_slots },
                        );
                    }
                    "VISUAL-LOCATION" => {
                        self.now(Prio::Num(0), Out::Medium, Event::FindLocation { spec });
                    }
                    "VISUAL" => {
                        self.visual_request(spec)?;
                    }
                    "MANUAL" => {
                        self.manual_request(spec)?;
                    }
                    "IMAGINAL" => {
                        // imaginal-request: busy requests are ignored
                        // (warning to stdout in Lisp, nothing in trace).
                        if !self.imag_busy {
                            self.imag_busy = true;
                            let delay = if self.cfg.vidt {
                                self.randomize_ms(self.cfg.imaginal_delay_ms)
                            } else {
                                self.cfg.imaginal_delay_ms
                            };
                            let at = self.time_ms + delay;
                            self.push(at, Prio::Num(0), Out::Never, Event::ImaginalDone { spec });
                        }
                    }
                    "IMAGINAL-ACTION" => {
                        self.apply_pending_imaginal_mod();
                        self.imaginal_action_request(spec)?;
                    }
                    "TRACKER" => {
                        if let NewChunk::Spec { slots, .. } = spec {
                            self.tracker_request(&slots)?;
                        }
                    }
                    "TEMPORAL" => {
                        self.temporal_request(spec)?;
                    }
                    "VOCAL" => {
                        self.vocal_request(spec)?;
                    }
                    "BLENDING" => {
                        // blending-request: clear the error flag, mark busy,
                        // and schedule the real work at priority -3000 so
                        // other 0-time buffer changes settle first
                        // (blending.lisp:465).
                        self.blending_busy = true;
                        self.blending_error = false;
                        self.now(Prio::Num(-3000), Out::Medium, Event::StartBlending { spec });
                    }
                    "GOAL" => {
                        if self.cfg.threads {
                            // Threaded goal request: create a fresh chunk
                            // (`CHUNKn`) and overwrite it into the
                            // multi-buffer while keeping the other goals.
                            let mut inst = self.instantiate("GOAL", spec)?;
                            let k = *self.name_count.entry("CHUNK".to_string()).or_insert(0);
                            self.name_count.insert("CHUNK".to_string(), k + 1);
                            inst.name = Some(format!("CHUNK{k}"));
                            self.now(
                                Prio::Num(-1000),
                                Out::Low,
                                Event::OverwriteBuffer {
                                    buffer: "GOAL".to_string(),
                                    chunk: NewChunk::Instance(inst),
                                    requested: true,
                                    shown: None,
                                    module: "GOAL".to_string(),
                                    copy_name: None,
                                },
                            );
                            return Ok(());
                        }
                        // goal-style-request: delay 0, priority -1000, so the
                        // new goal lands after the production's clears
                        // (`support/goal-style-module.lisp:245`). Request
                        // specs print slots-only (PORTING-NOTES #44); a whole-chunk
                        // reference (`+goal> =retrieval`) prints the
                        // referenced chunk's slots.
                        let shown = self.named_chunk_shown(&spec);
                        self.push(
                            self.time_ms,
                            Prio::Num(-1000),
                            // `schedule-set-buffer-chunk` defaults to
                            // `:output low` (`buffers.lisp:1403`), so the
                            // goal set prints at any detail level.
                            Out::Low,
                            Event::SetBuffer {
                                buffer: "GOAL".to_string(),
                                chunk: spec,
                                requested: true,
                                shown,
                            },
                        );
                    }
                    "AURAL-LOCATION" => {
                        self.now(Prio::Num(0), Out::Medium, Event::FindSound { spec });
                    }
                    "AURAL" => {
                        // `attend-sound` (`audio.lisp:1099`): resolve the
                        // audio-event chunk named by the request's `event`
                        // slot to its sound, go busy, and schedule
                        // `audio-encoding-complete` after the recode delay
                        // (`tone-recode-delay`).
                        let ename = match &spec {
                            NewChunk::Spec { slots, .. } => slots
                                .iter()
                                .find(|(s, _)| s == "EVENT")
                                .and_then(|(_, v)| match v {
                                    Value::Sym(s) => Some(s.clone()),
                                    _ => None,
                                }),
                            _ => None,
                        };
                        if let Some(ename) = ename {
                            let display = ename.clone();
                            let source =
                                self.audio_copied_from.get(&ename).cloned().unwrap_or(ename);
                            if let Some(idx) = self.sounds.iter().position(|s| s.ename == source) {
                                self.now(
                                    Prio::Num(0),
                                    Out::Medium,
                                    Event::AttendSound { idx, display },
                                );
                            }
                        }
                    }
                    "PRODUCTION" => {
                        // `procedural.lisp:2594`: the per-selection
                        // module-request to the `production` buffer. It has
                        // no visible effect (output nil).
                    }
                    got => {
                        return Err(unimpl(format!("request to buffer {got}")));
                    }
                }
            }
            Event::StartRetrieval { spec, var_slots } => {
                self.retrieval_pending = false;
                if show {
                    self.emit("DECLARATIVE", "start-retrieval".into());
                }
                // `retrieval-request-hook` → `al-chunk-retrieval-request`
                // (`associative-learning.lisp:644`): record the source-buffer
                // chunks the retrieval will learn from.
                if self.cfg.al_eal.is_some() {
                    self.al_last_contexts = self.al_source_buffer_chunks();
                }
                self.start_retrieval(spec, &var_slots)?;
            }
            Event::RetrievedChunk { name, gen } => {
                if gen != self.dm_gen {
                    return Ok(()); // aborted by a newer request: silent.
                }
                if show {
                    self.emit("DECLARATIVE", format!("RETRIEVED-CHUNK {name}"));
                }
                self.dm_busy = false;
                self.buffers.get_mut("RETRIEVAL").unwrap().busy = false;
                self.add_finst(&name);
                // `retrieved-chunk-hook` → `al-chunk-retrieved`
                // (`associative-learning.lisp:655`): learn from the retrieved
                // chunk and the context recorded at the request.
                if self.cfg.al_eal.is_some() {
                    let ctx = self.al_last_contexts.clone();
                    self.al_update(&name, &ctx);
                }
                // `set-buffer-chunk` copies the retrieved chunk (`buffer-copy`).
                // A reusable RETRIEVAL buffer names it `RETRIEVAL-CHUNK{n}`;
                // a production that mentions `=RETRIEVAL` turns reuse off
                // (`buffer-requires-copies`), so it gets the generic copy name
                // `{SOURCE}-{k}` instead.
                let copy = if self.model.no_reuse_buffers.iter().any(|b| b == "RETRIEVAL") {
                    self.copy_chunk(&name)
                } else {
                    let copy = self.reuse_buffer_name("RETRIEVAL");
                    self.copy_chunk_as(&name, &copy);
                    copy
                };
                self.now(
                    Prio::Max,
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "RETRIEVAL".to_string(),
                        chunk: NewChunk::Named(copy),
                        requested: true,
                        shown: Some(name),
                    },
                );
            }
            Event::RetrievalFailure { gen } => {
                if gen != self.dm_gen {
                    return Ok(());
                }
                if show {
                    self.emit("DECLARATIVE", "RETRIEVAL-FAILURE".into());
                }
                self.dm_busy = false;
                let b = self.buffer_mut("RETRIEVAL");
                b.busy = false;
                b.failed = true;
                self.changed();
            }
            Event::StartBlending { spec } => {
                if show {
                    self.emit("BLENDING", "START-BLENDING".into());
                }
                self.start_blending(&spec)?;
            }
            Event::BlendingComplete { spec } => {
                if show {
                    self.emit("BLENDING", "BLENDING-COMPLETE".into());
                }
                // blending-complete: free the module, then place the chunk
                // (priority -1000, default `low` output).
                self.blending_busy = false;
                self.now(
                    Prio::Num(-1000),
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "BLENDING".to_string(),
                        chunk: spec,
                        requested: true,
                        shown: None,
                    },
                );
            }
            Event::BlendingFailure => {
                if show {
                    self.emit("BLENDING", "BLENDING-FAILURE".into());
                }
                self.blending_busy = false;
                self.blending_error = true;
                let b = self.buffer_mut("BLENDING");
                b.busy = false;
                b.failed = true;
                self.changed();
            }
            Event::ConflictResolution => {
                if show {
                    self.emit("PROCEDURAL", "CONFLICT-RESOLUTION".into());
                }
                self.conflict_resolution()?;
            }
            Event::ProductionSelected { prod, bindings } => {
                let name = self.model.productions[prod].name.clone();
                self.last_event_bindings = Some((name.clone(), bindings.clone()));
                if show {
                    self.emit("PROCEDURAL", format!("PRODUCTION-SELECTED {name}"));
                }
                // `pbreak` (`procedural.lisp:2318`): print the instantiation,
                // emit a break event, and halt the run before the reads.
                if self.zz_breaks.iter().any(|b| b == &name) {
                    if !self.zz_break_quiet {
                        self.zz_print_instantiation(prod, &bindings);
                    }
                    self.emit("------", format!("BREAK-EVENT PRODUCTION {name}"));
                    self.zz_break_hit = true;
                }
                // Utility-learning history records selections.
                if self.cfg.ul {
                    self.history.push((prod, self.time_ms));
                }
                // `procedural.lisp:2594`: selecting a production schedules an
                // invisible `module-request` to the `production` buffer (its
                // `details` is the production name). It has no effect on the
                // trace, but ACT-R counts it as an event.
                self.now(
                    Prio::Max,
                    Out::Never,
                    Event::ModuleRequest {
                        buffer: "PRODUCTION".to_string(),
                        spec: NewChunk::Spec {
                            ctype: "CHUNK".to_string(),
                            slots: Vec::new(),
                            neg: Vec::new(),
                            rels: Vec::new(),
                            bare: true,
                        },
                        var_slots: Vec::new(),
                        trace: false,
                    },
                );
                // Reads and queries in LHS order, implicit queries last,
                // then firing at +dat.
                for item in self.model.productions[prod].lhs.clone() {
                    match item {
                        LhsItem::Match { buffer, .. } => {
                            self.now(Prio::Num(0), Out::Always, Event::BufferRead { buffer });
                        }
                        LhsItem::Query { buffer, .. } => {
                            self.now(Prio::Num(0), Out::Always, Event::QueryBuffer { buffer });
                        }
                        LhsItem::Eval { .. } | LhsItem::Bind { .. } | LhsItem::MvBind { .. } => {}
                    }
                }
                for buffer in self.exec_prods[prod].implicit_queries.clone() {
                    self.now(Prio::Num(0), Out::Always, Event::QueryBuffer { buffer });
                }
                let action_ms = self.model.productions[prod]
                    .at
                    .map(|s| (s * 1000.0).round() as i64)
                    .unwrap_or(self.cfg.dat_ms);
                let action_ms = if self.cfg.vpft {
                    self.randomize_ms(action_ms)
                } else {
                    action_ms
                };
                let at = self.time_ms + action_ms;
                self.push(
                    at,
                    Prio::Num(0),
                    Out::Low,
                    Event::ProductionFired { prod, bindings },
                );
            }
            Event::BufferRead { buffer } => {
                if show {
                    self.emit("PROCEDURAL", format!("BUFFER-READ-ACTION {buffer}"));
                }
            }
            Event::QueryBuffer { buffer } => {
                if show {
                    self.emit("PROCEDURAL", format!("QUERY-BUFFER-ACTION {buffer}"));
                }
            }
            Event::ProductionFired { prod, bindings } => {
                let name = self.model.productions[prod].name.clone();
                self.last_event_bindings = Some((name.clone(), bindings.clone()));
                if show {
                    self.emit("PROCEDURAL", format!("PRODUCTION-FIRED {name}"));
                }
                self.fire(prod, &bindings)?;
                // `(spp <prod> :reward R)`: propagate the reward at fire time
                // (after the RHS actions), like ACT-R's `learn-parameters`
                // (`utility-and-reward-1.lisp:475`), which schedules
                // `trigger-reward` with `:priority :min` — so the goal's
                // `set-buffer-chunk` (priority -1000, `goal-style-module.lisp:245`)
                // lands first. bst's `read-done` trace shows the goal set before
                // `propagate-reward`.
                if let Some(r) = self.model.productions[prod].reward.clone() {
                    self.now(
                        Prio::Num(-1001),
                        Out::Medium,
                        Event::PropagateReward { value: r },
                    );
                }
            }
            Event::ProcDisplay => {
                self.display_queued_ms = None;
                if show {
                    self.emit("VISION", "proc-display".into());
                }
                self.proc_display()?;
            }
            Event::VisiconUpdate => {
                if show {
                    self.emit("VISION", "visicon-update".into());
                }
                // `monitor-visicon-updates` (`perceptual-history.lisp`).
                self.record_visicon_snapshot();
                // A late print the task scheduled for this instant runs at the
                // very end of the display wave, after the wave's own
                // `conflict-resolution` (which the buffer stuffing queues).
                let now = self.time_ms;
                let mut late = Vec::new();
                self.pending_late_prints.retain(|p| {
                    if p.at_ms <= now {
                        late.push(p.clone());
                        false
                    } else {
                        true
                    }
                });
                for p in late {
                    let prio = if p.after_update {
                        Prio::Num(-1)
                    } else {
                        Prio::Min
                    };
                    self.push_m(
                        p.at_ms,
                        prio,
                        Out::Always,
                        true,
                        Event::PrintVisicon {
                            at_ms: p.at_ms,
                            seq: p.seq,
                        },
                    );
                }
            }
            Event::PrintVisicon { at_ms, seq } => {
                // A task's `print-visicon` inside a run: pure output, so it
                // emits no trace line of its own, just the table.
                self.print_visicon_rpc(true, Some((at_ms, seq)));
            }
            Event::UnlockVision => {
                // `unlock-vision` (`vision.lisp:1553`): drop the lock, and if a
                // proc-display was blocked while it was held, run it now (no
                // `proc-display` trace line — the ref has only `visicon-update`).
                self.visual_lock = false;
                if std::mem::take(&mut self.blocked_proc_display) {
                    self.proc_display()?;
                    // The re-run consumed the wave; a new op at this time may
                    // schedule its own (`internal-proc-display` clears
                    // `scheduled-proc-display`, `vision.lisp:1577`).
                    self.display_queued_ms = None;
                }
            }
            Event::FindLocation { spec } => {
                if show {
                    self.emit("VISION", "Find-location".into());
                }
                self.find_location(spec)?;
            }
            Event::FindLocFailure => {
                if show {
                    self.emit("VISION", "find-loc-failure".into());
                }
                // Buffer failure was set when the search failed.
            }
            Event::ImaginalDone { spec } => {
                // `+imaginal> =loc` copies a visual-location chunk's slots, not
                // its name. An image's location chunk has no color and prints
                // `kind height width screen-y screen-x value distance size`.
                let spec = if let NewChunk::Named(n) = &spec {
                    let base = self.vl_copied_from.get(n).cloned();
                    base.and_then(|base| {
                        self.features
                            .iter()
                            .find(|f| f.name == base && f.kind == "IMAGE")
                            .map(|f| NewChunk::Spec {
                                ctype: String::new(),
                                slots: vec![
                                    ("KIND".to_string(), Value::Sym("IMAGE".to_string())),
                                    ("HEIGHT".to_string(), Value::Int(f.height)),
                                    ("WIDTH".to_string(), Value::Int(f.width)),
                                    ("SCREEN-Y".to_string(), Value::Int(f.y)),
                                    ("SCREEN-X".to_string(), Value::Int(f.x)),
                                    ("VALUE".to_string(), Value::Sym("IMAGE".to_string())),
                                    ("DISTANCE".to_string(), Value::Int(1080)),
                                    ("SIZE".to_string(), Value::Num(f.size())),
                                ],
                                neg: Vec::new(),
                                rels: Vec::new(),
                                bare: true,
                            })
                    })
                } else {
                    None
                }
                .unwrap_or(spec);
                // set-imaginal-free + goal-style-request, both invisible.
                self.imag_busy = false;
                self.now(
                    Prio::Num(-1000),
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "IMAGINAL".to_string(),
                        chunk: spec,
                        requested: true,
                        shown: None,
                    },
                );
            }
            Event::ImaginalFree => {
                // `set-imaginal-free` (`imaginal.lisp`, maintenance event).
                self.imag_busy = false;
                self.buffer_mut("IMAGINAL").busy = false;
            }
            Event::ImaginalError => {
                // set-imaginal-error: free the module and mark the imaginal
                // buffer failed (`imaginal.lisp:296`).
                self.imag_busy = false;
                let b = self.buffer_mut("IMAGINAL");
                b.busy = false;
                b.failed = true;
                self.changed();
            }
            Event::StartTracking => {
                if show {
                    self.emit("VISION", "START-TRACKING".into());
                }
                let feat = self
                    .last_attention
                    .as_ref()
                    .and_then(|loc| self.resolve_loc(loc).ok())
                    .map(|i| self.features[i].name.clone());
                if let Some(name) = feat {
                    self.tracked = Some(name);
                    self.tracked_lastloc = None;
                    // `change-state :exec 'BUSY` (`vision.lisp:2677`): the
                    // vision module stays busy while tracking, so
                    // `?visual> state free` fails.
                    self.visual_busy = true;
                    self.update_tracking(false);
                }
            }
            Event::MoveAttention { loc } => {
                if show {
                    self.emit("VISION", format!("Move-attention {loc} NIL"));
                }
                self.last_attention = Some(loc.clone());
                self.visual_busy = true;
                // A new attention shift supersedes any in-flight
                // `update-attended-loc` re-encode: ACT-R blocks the window
                // clear's `proc-display` behind the vision lock, so the
                // re-encode never runs and the module stays busy until this
                // shift completes (Sperling: the 0.941 clear vs the 0.975
                // move-attention, freeing vision at 1.060).
                self.queue.retain(|q| {
                    !matches!(
                        q.event,
                        Event::EncodingComplete {
                            requested: false,
                            ..
                        }
                    )
                });
                if self.cfg.emma {
                    // EMMA replaces the fixed-latency encoding with its
                    // saccade machinery (`emma-attention-hook`).
                    let end = self.emma_loc_xyz(&loc);
                    self.emma_attention_hook(end);
                } else {
                    let lat = self.randomize_ms(self.cfg.attention_ms);
                    let at = self.time_ms + lat;
                    self.push(
                        at,
                        Prio::Num(0),
                        Out::Medium,
                        Event::EncodingComplete {
                            loc,
                            requested: true,
                        },
                    );
                }
            }
            Event::EncodingComplete { loc, requested } => {
                if show {
                    self.emit("VISION", format!("Encoding-complete {loc} NIL"));
                }
                self.encoding_complete(loc, requested)?;
            }
            Event::EncodingMissing { loc } => {
                if show {
                    self.emit("VISION", format!("Encoding-complete {loc} NIL"));
                }
                self.encoding_no_object()?;
            }
            Event::EmmaPrepComplete { trgt } => {
                if show {
                    self.emit(
                        "EMMA",
                        format!("Preparation-complete {}", lisp_sym_opt(&trgt)),
                    );
                }
                self.emma_preparation_complete();
            }
            Event::EmmaInitComplete => {
                if show {
                    self.emit("EMMA", "INITIATION-COMPLETE".into());
                }
                // `initiation-complete`: free the processor state.
            }
            Event::EmmaEyeMoveComplete { sacc } => {
                if show {
                    self.emit(
                        "EMMA",
                        format!(
                            "Complete-eye-movement {} {}",
                            lisp_sym_opt(&sacc.trgt),
                            fmt_vec3(sacc.new_loc)
                        ),
                    );
                }
                self.emma_complete_eye_move(sacc);
            }
            Event::VisualClear => {
                if show {
                    self.emit("VISION", "CLEAR".into());
                }
                self.visual_busy = false;
                // `clear :after` (`vision.lisp:2886`) drops the attended
                // object and the current marker, so a later window change
                // no longer re-encodes the old location.
                self.last_attention = None;
                // pm-module clear also frees preparation state 50 ms
                // later via an (at most high-detail) change-state event;
                // executing it wakes an armed conflict-resolution.
                // Details inferred from the scheduled params; no test
                // shows this line yet (high-detail visual clears only).
                let at = self.time_ms + self.cfg.feat_prep_ms;
                self.push(
                    at,
                    Prio::Num(0),
                    Out::Always,
                    Event::ChangeState {
                        module: "VISION".to_string(),
                    },
                );
            }
            Event::Dummy => {}
            Event::AdaptiveRow => self.adaptive_noise_row(),
            Event::MergeDm { specs } => {
                for slots in specs {
                    let inst = ChunkInst {
                        name: None,
                        ctype: String::new(),
                        slots,
                        bare: false,
                    };
                    self.merge_into_dm(inst, "MERGE-DM");
                }
            }
            Event::Break { details } => {
                self.emit("------", format!("BREAK-EVENT {details}"));
                self.stopped = true;
            }
            Event::Emit { module, line } => {
                if show {
                    self.emit(&module, line);
                }
            }
            Event::ZzKeyPrep {
                cmd,
                style,
                key,
                hand,
                finger,
                r,
                theta,
                loc,
                delay_ms,
                req_ms,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_for(&style, hand)
                        ),
                    );
                }
                let _ = theta;
                let _ = cmd;
                self.man_prep_busy = false;
                let init = self.cfg.init_ms;
                let burst = self.cfg.burst_ms;
                let kct = self.cfg.zz_kct_ms;
                let mv = (burst as f32 / 1000.0).max(
                    (self.cfg.touch_peck_coeff * (r + 0.5).log2())
                        .max(self.cfg.min_fitts_ms as f32 / 1000.0),
                );
                let move_ms = (mv * 1000.0).round() as i64;
                let strike = self.zz_movement_strike_time(move_ms, r);
                let (exec_ms, exec2_ms, finish_ms) = match style.as_str() {
                    // `hold-peck` (`motor-extension.lisp:1909`): exec is the
                    // strike time, finish is the full move time (they differ
                    // only when `peck-strike-distance` is set).
                    "HOLD-PECK" => (init + strike, None, init + move_ms),
                    // Base `peck`/`punch` only press; the extended styles add
                    // the automatic release (`set-finger-up`).
                    "PECK" if !self.cfg.zz_ext => (init + strike, None, init + move_ms + burst),
                    "PECK" => (
                        init + strike,
                        Some(init + move_ms + self.cfg.zz_krt_ms),
                        init + move_ms + burst,
                    ),
                    // `peck-recoil` (`motor-extension.lisp:1792`): finish adds a
                    // second full move time (strike out and recoil back).
                    "PECK-RECOIL" => (
                        init + strike,
                        Some(init + move_ms + self.cfg.zz_krt_ms),
                        init + move_ms + burst + move_ms,
                    ),
                    "HOLD-PUNCH" => (init + kct, None, init + burst),
                    // `delayed-punch` (`motor-extension.lisp:2657`): the release
                    // (exec2) and finish are pushed out by the `delay` slot.
                    "DELAYED-PUNCH" => (
                        init + kct,
                        Some(init + kct + delay_ms),
                        init + kct + delay_ms + burst,
                    ),
                    "PUNCH" if !self.cfg.zz_ext => (init + kct, None, init + burst + burst),
                    "PUNCH" => (
                        init + kct,
                        Some(init + self.cfg.zz_krt_ms),
                        init + burst + burst,
                    ),
                    "RELEASE" => (init + self.cfg.zz_krt_ms, None, init + burst),
                    // `release-recoil` (`motor-extension.lisp:1991`): a peck
                    // that releases at exec and recoils home.
                    "RELEASE-RECOIL" => (init + self.cfg.zz_krt_ms, None, init + move_ms + burst),
                    // `point-finger` (`motor-extension.lisp:2036`): exec is
                    // `max(init, move-time)`, finish `init + move-time`.
                    "POINT-FINGER" => (init.max(move_ms), None, init + move_ms),
                    _ => (init, None, init + burst),
                };
                self.man_exec_queue.push(ZzPending {
                    key: key.clone(),
                    req_ms,
                    hand,
                    both: false,
                    style: style.clone(),
                    exec_ms,
                    finish_ms,
                    kind: ZzKind::Key {
                        finger,
                        exec2_ms,
                        // `point-finger` and the release styles produce no
                        // `output-key`; `release`/`release-recoil` emit a
                        // `release-key` at exec instead.
                        output: !matches!(
                            style.as_str(),
                            "POINT-FINGER" | "RELEASE" | "RELEASE-RECOIL"
                        ),
                        release_exec: matches!(style.as_str(), "RELEASE" | "RELEASE-RECOIL"),
                    },
                });
                // `move-a-finger` (`motor.lisp:712`) shifts the finger's
                // *offset*; the hand loc is unchanged (only `move-hand` moves
                // it). The next same/move test reads `hand-loc + finger-offset`
                // (`motor-extension.lisp:2120`).
                let target = loc.or_else(|| key_xy(&key));
                if let Some(kl) = target {
                    // `peck`/`point-finger`/`hold-peck` leave the finger at the
                    // key; `peck-recoil` returns it to where it started
                    // (`updated-pos` is the start, `motor-extension.lisp:1776`),
                    // and `release` doesn't move it.
                    if !matches!(style.as_str(), "RELEASE" | "PECK-RECOIL") {
                        let (hx, hy) = self.zz_hands[hand].loc;
                        self.zz_hands[hand].offsets[finger] = (kl.0 - hx, kl.1 - hy);
                    }
                }
                self.zz_maybe_execute();
            }
            Event::ZzKeyOutput {
                key,
                down,
                hand,
                finger,
            } => {
                // `set-finger-down`/`set-finger-up` (`motor-extension.lisp:1579`/
                // `:1593`) update the tracker and then `notify-motor-device`,
                // which *schedules* `output-key`/`release-key`
                // (`keyboard.lisp:503`) — so the trace is a separate event whose
                // sequence lands after events already queued at this time.
                let name = if down { "output-key" } else { "release-key" };
                self.now(
                    Prio::Num(0),
                    Out::Always,
                    Event::Emit {
                        module: "KEYBOARD".to_string(),
                        line: format!("{name} {} {}", self.model.name, key.to_lowercase()),
                    },
                );
                // The finger goes down/up at execution time (`set-finger-down`
                // / `set-finger-up`).
                self.zz_hands[hand].down[finger] = down;
                // Base direct `peck`/`punch` presses are reported once; only
                // the extended styles leave the finger held for a later
                // release event.
                self.zz_hands[hand].held[finger] = if down && self.cfg.zz_ext {
                    Some(key.clone())
                } else {
                    None
                };
                if down {
                    self.zz_data
                        .push(("PRESSED".to_string(), self.time_ms, key));
                } else {
                    self.zz_data
                        .push(("RELEASED".to_string(), self.time_ms, key));
                }
            }
            Event::ZzHandPrep {
                req_ms,
                r,
                theta,
                to_mouse,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_for("HAND-PLY", 1)
                        ),
                    );
                }
                self.man_prep_busy = false;
                let init = self.cfg.init_ms;
                let burst = self.cfg.burst_ms;
                let fit = (self.cfg.touch_cursor_coeff * (r / 4.0 + 0.5).log2())
                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                let exec = init + (fit * 1000.0).round() as i64;
                let finish = exec + burst;
                self.man_exec_queue.push(ZzPending {
                    key: String::new(),
                    req_ms,
                    hand: 1,
                    both: false,
                    style: "HAND-PLY".to_string(),
                    exec_ms: exec,
                    finish_ms: finish,
                    kind: ZzKind::Hand { to_mouse, r, theta },
                });
                self.zz_maybe_execute();
            }
            Event::ZzHandPlyPrep {
                class,
                hand,
                r,
                theta,
                r_str,
                theta_str,
                target,
                twidth,
                req_ms,
                deferred,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand(&class, hand, false)
                        ),
                    );
                }
                let _ = theta;
                self.man_prep_busy = false;
                let init = self.cfg.init_ms;
                let burst = self.cfg.burst_ms;
                // `hand-ply` (`motor.lisp:1084`): exec = init + fitts with the
                // mouse coefficient and the request's target width (the keypad
                // passes 4.0, `keyboard.lisp:637`); finish = exec + burst.
                let fit = (self.cfg.touch_cursor_coeff * (r / twidth + 0.5).log2())
                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                let exec = init + (fit * 1000.0).round() as i64;
                let finish = exec + burst;
                let pending = ZzPending {
                    key: String::new(),
                    req_ms,
                    hand,
                    both: false,
                    style: class,
                    exec_ms: exec,
                    finish_ms: finish,
                    kind: ZzKind::HandPly {
                        target,
                        r: r_str,
                        theta: theta_str,
                    },
                };
                if deferred {
                    // `preparation-complete` (`motor-extension.lisp:1019`): a
                    // non-immediate movement is not queued, and its processor is
                    // freed here rather than at `initiation-complete`.
                    self.zz_prepared = Some(pending);
                    if self.cfg.zz_two_proc {
                        self.man_proc[hand] = false;
                    } else {
                        self.man_proc[0] = false;
                        self.man_proc[1] = false;
                    }
                } else {
                    self.man_exec_queue.push(pending);
                    self.zz_maybe_execute();
                }
            }
            Event::ZzAllHomePrep { req_ms, hands, r } => {
                let both = hands.len() == 2;
                let hand = if both { 1 } else { hands[0] };
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand("ALL-FINGERS-TO-HOME", hand, both)
                        ),
                    );
                }
                self.man_prep_busy = false;
                let init = self.cfg.init_ms;
                // `all-fingers-to-home` (`motor-extension.lisp:2349`): exec is
                // `max(init, randomize(max(.2, fitts(mouse-coeff, r, 4.0))))`;
                // finish is `init + exec`.
                let fit = (self.cfg.touch_cursor_coeff * (r / 4.0 + 0.5).log2())
                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                let m = 0.2f32.max(fit);
                let exec = init.max((m * 1000.0).round() as i64);
                let finish = init + exec;
                self.man_exec_queue.push(ZzPending {
                    key: String::new(),
                    req_ms,
                    hand,
                    both,
                    style: "ALL-FINGERS-TO-HOME".to_string(),
                    exec_ms: exec,
                    finish_ms: finish,
                    kind: ZzKind::AllHome { hands },
                });
                self.zz_maybe_execute();
            }
            Event::ZzReleaseAllPrep { req_ms, hands } => {
                let both = hands.len() == 2;
                let hand = if both { 1 } else { hands[0] };
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand("RELEASE-ALL-FINGERS", hand, both)
                        ),
                    );
                }
                self.man_prep_busy = false;
                // `release-all-fingers` (`motor-extension.lisp:2577`): exec is
                // `init + key-release-time`, finish is `init + 2*burst`.
                let exec = self.cfg.init_ms + self.cfg.zz_krt_ms;
                let finish = self.cfg.init_ms + 2 * self.cfg.burst_ms;
                self.man_exec_queue.push(ZzPending {
                    key: String::new(),
                    req_ms,
                    hand,
                    both,
                    style: "RELEASE-ALL-FINGERS".to_string(),
                    exec_ms: exec,
                    finish_ms: finish,
                    kind: ZzKind::ReleaseAll { hands },
                });
                self.zz_maybe_execute();
            }
            Event::ZzMousePrep {
                cmd,
                req_ms,
                loc,
                fit_ms,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix()
                        ),
                    );
                }
                self.man_prep_busy = false;
                // The manual buffer stays busy from preparation through the
                // movement's finish: `?manual> state free` is the *buffer*
                // state (ACT-R frees it at `finish-movement`, not at
                // `initiation-complete`). Without a `zz_ext` model the buffer
                // state is read from `man_exec_busy`, so set it here as
                // `TouchPrep` does.
                self.man_exec_busy = true;
                if self.cfg.zz_two_exec {
                    self.man_exec[1] = true;
                } else {
                    self.man_exec[0] = true;
                    self.man_exec[1] = true;
                }
                let style = self.zz_last_style.clone().map(|x| x.0).unwrap_or_default();
                let init = self.cfg.init_ms;
                let burst = self.cfg.burst_ms;
                let (exec_ms, finish_ms, exec2_ms) = match cmd.as_str() {
                    "MOVE-CURSOR" => {
                        // The request computed the Fitts time from the target's
                        // approach width; fall back to a plain pixel ratio if
                        // the target geometry was unavailable.
                        let e = init
                            + fit_ms.unwrap_or_else(|| {
                                let (tx, ty, _) = loc.clone().unwrap_or((0, 0, String::new()));
                                let (cx, cy) = (self.zz_mouse_pos.0, self.zz_mouse_pos.1);
                                let r = (((tx - cx) as f32).powi(2) + ((ty - cy) as f32).powi(2))
                                    .sqrt();
                                let mut coef = self.cfg.touch_cursor_coeff;
                                if let Some(d) = self.cfg.zz_cdd {
                                    if self.zz_mouse_down {
                                        coef *= 2.0f32.powf(d as f32);
                                    }
                                }
                                let fit = (coef * (r / 18.85 + 0.5).log2())
                                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                                (fit * 1000.0).round() as i64
                            });
                        (e, e + burst, None)
                    }
                    "CLICK-MOUSE" => (
                        init + self.cfg.zz_kct_ms,
                        init + burst + burst,
                        // Only the extended-motor-actions `punch` queues an
                        // automatic release (`set-finger-up`,
                        // `motor-extension.lisp:1716`); the base `punch`
                        // queues just its press notification
                        // (`motor.lisp:955`), so a base `click-mouse` never
                        // releases and never wakes conflict resolution.
                        self.cfg.zz_ext.then_some(init + self.cfg.zz_krt_ms),
                    ),
                    "HOLD-MOUSE" => (init + self.cfg.zz_kct_ms, init + burst, None),
                    "RELEASE-MOUSE" => (init + self.cfg.zz_krt_ms, init + burst, None),
                    _ => (init, init + burst, None),
                };
                let t = self.time_ms;
                self.push(
                    t + init,
                    Prio::Num(0),
                    Out::Always,
                    Event::InitiationComplete {
                        req_ms,
                        hand: 1,
                        both: false,
                        style: style.clone(),
                    },
                );
                self.push(
                    t + exec_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::ZzMouseNotify {
                        cmd: cmd.clone(),
                        req_ms,
                        loc: loc.clone(),
                    },
                );
                if let Some(e2) = exec2_ms {
                    self.push(
                        t + e2,
                        Prio::Num(0),
                        Out::Always,
                        Event::ZzMouseRelease { req_ms },
                    );
                }
                self.push(
                    t + finish_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::FinishMovement {
                        req_ms,
                        hand: 1,
                        both: false,
                        style,
                    },
                );
            }
            Event::ZzMouseNotify { cmd, req_ms, loc } => {
                let _ = req_ms;
                let (mx, my, mz) = self.zz_mouse_pos;
                match cmd.as_str() {
                    "MOVE-CURSOR" => {
                        let (tx, ty, _) = loc.clone().unwrap_or((0, 0, String::new()));
                        self.zz_mouse_pos = (tx, ty, 1080);
                        // The device notification is a high-detail trace line
                        // (the refs trace `MOUSE` only at `:trace-detail high`).
                        if show {
                            self.emit(
                                "MOUSE",
                                format!(
                                    "move-cursor {} mouse ({} {} {})",
                                    self.model.name, tx, ty, 1080
                                ),
                            );
                        }
                    }
                    "CLICK-MOUSE" | "HOLD-MOUSE" => {
                        self.zz_mouse_down = true;
                        self.record_click_action();
                        if show {
                            self.emit(
                                "MOUSE",
                                format!(
                                    "click-mouse {} ({} {} {}) INDEX",
                                    self.model.name, mx, my, mz
                                ),
                            );
                        }
                        self.zz_data.push((
                            "CLICKED".to_string(),
                            self.time_ms,
                            format!("({mx} {my} {mz})"),
                        ));
                    }
                    "RELEASE-MOUSE" => {
                        self.zz_mouse_down = false;
                        if show {
                            self.emit(
                                "MOUSE",
                                format!(
                                    "release-mouse {} ({} {} {}) index",
                                    self.model.name, mx, my, mz
                                ),
                            );
                        }
                        self.zz_data.push((
                            "UNCLICKED".to_string(),
                            self.time_ms,
                            format!("({mx} {my} {mz})"),
                        ));
                    }
                    _ => {}
                }
            }
            Event::ZzMouseRelease { req_ms } => {
                let _ = req_ms;
                self.zz_mouse_down = false;
                let (mx, my, mz) = self.zz_mouse_pos;
                if show {
                    self.emit(
                        "MOUSE",
                        format!(
                            "release-mouse {} ({} {} {}) INDEX",
                            self.model.name, mx, my, mz
                        ),
                    );
                }
                self.zz_data.push((
                    "UNCLICKED".to_string(),
                    self.time_ms,
                    format!("({mx} {my} {mz})"),
                ));
                self.run_scripted_click();
            }
            Event::TrackerUpdate { gen } => {
                if gen == self.tracker_gen {
                    if show {
                        if let Some(t) = self.trackers.first() {
                            self.emit(
                                "TRACKER",
                                format!("update-tracker {} {}", t.name, t.control_slot),
                            );
                        }
                    }
                    self.tracker_update()?;
                } else {
                    self.event_silent = true;
                }
            }
            Event::Choose => {
                self.tracker_set_prompt("CHOOSE");
            }
            Event::ModFocus { slots } => {
                if show {
                    self.emit("GOAL", "GOAL-MODIFICATION".into());
                }
                if let Some(c) = self.buffers.get_mut("GOAL").and_then(|b| b.chunk.as_mut()) {
                    for (k, v) in slots {
                        match c.slots.iter_mut().find(|(s, _)| s == &k) {
                            Some((_, old)) => *old = v.clone(),
                            None => c.slots.push((k.clone(), v.clone())),
                        }
                    }
                }
                self.changed();
            }
            Event::ModifyVisicon { name, x, y } => {
                if let Some(f) = self.features.iter_mut().find(|f| f.name == name) {
                    f.x = x;
                    f.y = y;
                    f.tstamp_ms = self.time_ms;
                }
                // `modify-visicon-features` triggers a visicon update and
                // `proc-display` (`vision.lisp`), re-stuffing the location.
                self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
            }
            Event::TouchPrep { plan } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("PREPARATION-COMPLETE {}", Self::secs_str(plan.req_ms)),
                    );
                }
                self.man_prep_busy = false;
                self.man_exec_busy = true;
                let t = self.time_ms;
                self.push(
                    t + plan.init_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::TouchInit { plan: plan.clone() },
                );
                self.push(
                    t + plan.exec_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::TouchSignal { plan: plan.clone() },
                );
                if plan.up_at_exec {
                    self.push(
                        t + plan.exec_ms,
                        Prio::Num(-1),
                        Out::Never,
                        Event::TouchUp { plan: plan.clone() },
                    );
                } else if plan.up_at_exec_burst {
                    self.push(
                        t + plan.exec_ms + self.cfg.burst_ms,
                        Prio::Num(-1),
                        Out::Never,
                        Event::TouchUp { plan: plan.clone() },
                    );
                }
                self.push(
                    t + plan.finish_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::TouchFinish { plan },
                );
            }
            Event::TouchInit { plan } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("INITIATION-COMPLETE {}", Self::secs_str(plan.req_ms)),
                    );
                }
                if plan.down_at_init {
                    if let (Some(h), Some(f)) = (
                        Self::touch_hand_idx(&plan.hand),
                        Self::touch_finger_idx(&plan.finger),
                    ) {
                        self.touch_fingers[h][f].2 = 0;
                    }
                }
            }
            Event::TouchSignal { plan } => {
                self.emit(&plan.signal_module, plan.signal.clone());
                if !plan.monitor.is_empty() {
                    self.out_lines.push(plan.monitor.clone());
                }
                if let (Some(h), Some(f)) = (
                    Self::touch_hand_idx(&plan.hand),
                    Self::touch_finger_idx(&plan.finger),
                ) {
                    if plan.down_at_exec {
                        self.touch_fingers[h][f].2 = 0;
                    }
                    if plan.up_at_exec {
                        self.touch_fingers[h][f].2 = 1;
                    }
                    if let Some((x, y)) = plan.move_to {
                        self.touch_fingers[h][0] = (x, y, self.touch_fingers[h][0].2);
                    }
                    if plan.put_hand {
                        self.start_hand_at_touch(&plan.hand);
                    }
                }
            }
            Event::TouchUp { plan } => {
                if let (Some(h), Some(f)) = (
                    Self::touch_hand_idx(&plan.hand),
                    Self::touch_finger_idx(&plan.finger),
                ) {
                    self.touch_fingers[h][f].2 = 1;
                }
            }
            Event::TouchFinish { plan } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("FINISH-MOVEMENT {}", Self::secs_str(plan.req_ms)),
                    );
                }
                self.man_exec_busy = false;
            }
            Event::TemporalTick { gen, details } => {
                if gen == self.temporal_gen {
                    if show {
                        self.emit("TEMPORAL", details);
                    }
                    self.temporal_tick(show)?;
                } else {
                    // Cancelled by a newer request (`delete-event`): it must
                    // not wake conflict resolution.
                    self.event_silent = true;
                }
            }
            Event::TemporalClear => {
                if show {
                    self.emit("TEMPORAL", "Clear".into());
                }
                let b = self.buffer_mut("TEMPORAL");
                b.chunk = None;
                b.failed = false;
            }
            Event::PropagateReward { value } => {
                self.reward_seen = true;
                if show {
                    self.emit(
                        "UTILITY",
                        format!("PROPAGATE-REWARD {}", value.trace_text()),
                    );
                }
                // `production-history-reward-markers` (a reward-notify hook).
                let reward_num = match &value {
                    Reward::Num(v) => Some(*v),
                    Reward::Other(_) => None,
                };
                self.p_hist_reward(reward_num);
                match value {
                    Reward::Num(value) => {
                        // Rewards apply only with subsymbolics + learning on;
                        // otherwise silently ignored (Lisp warns to stdout).
                        if self.cfg.esc && self.cfg.ul {
                            if self.cfg.ult {
                                self.out(format!(
                                    " Utility updates with Reward = {}   alpha = {}",
                                    lisp_s_f32(value),
                                    lisp_s_f32(self.cfg.alpha)
                                ));
                            }
                            for (prod, at) in self.history.clone() {
                                let dt = (self.time_ms - at) as f32 / 1000.0;
                                let r = value - dt;
                                let u_old = self.utilities[prod];
                                let alpha = self.cfg.alpha;
                                let u_new = u_old + alpha * (r - u_old);
                                self.utilities[prod] = u_new;
                                if self.cfg.ult {
                                    let pname = self.model.productions[prod].name.clone();
                                    self.out(format!("  Updating utility of production {pname}"));
                                    self.out(format!(
                                        "   U(n-1) = {}   R(n) = {} [{} - {} seconds since selection]",
                                        lisp_s_f32(u_old),
                                        lisp_s_f32(r),
                                        lisp_s_f32(value),
                                        lisp_s_f32(dt)
                                    ));
                                    self.out(format!("   U(n) = {}", lisp_s_f32(u_new)));
                                }
                            }
                            self.history.clear();
                        }
                    }
                    // `propagate-reward`'s non-numeric branch
                    // (`utility-and-reward-1.lisp:463`): any non-nil non-number
                    // reward clears the utility-learning history, unconditionally
                    // (the message only with `:ult`).
                    Reward::Other(_) => {
                        if self.cfg.ult {
                            self.out(
                                "  Non-numeric reward clears utility learning history.".to_string(),
                            );
                        }
                        self.history.clear();
                    }
                }
            }
            Event::ChangeState { module } => {
                if show {
                    if module == "MANUAL" {
                        self.emit("MOTOR", "CHANGE-STATE LAST NONE PREP FREE PROC FREE".into());
                    } else {
                        self.emit(
                            &module,
                            "CHANGE-STATE :LAST NONE :PREP FREE :PROC FREE".into(),
                        );
                    }
                }
                // `clear`'s scheduled `change-state` frees the vision module's
                // preparation state (`general-pm.lisp:333`), and the manual
                // module's prep/processor state.
                if module == "VISION" {
                    self.visual_prep_busy = false;
                    self.changed();
                } else if module == "MANUAL" {
                    self.man_prep_busy = false;
                    self.man_exec_busy = false;
                    self.man_exec[0] = false;
                    self.man_exec[1] = false;
                    self.changed();
                }
            }
            Event::PressKey {
                key,
                req_ms,
                isa: _isa,
            } => {
                if show {
                    self.emit("MOTOR", format!("PRESS-KEY KEY {key}"));
                }
                // Preparation features. `key->cmd` maps home-row keys to the
                // `punch` style (`motor.lisp:935`, slots hand/finger) and the
                // rest to `peck-recoil` (slots hand/finger/r/theta). Fresh
                // movement uses `num-to-prepare` (`general-pm.lisp:790`); a
                // repeat uses the style's `feat-differences`
                // (`motor.lisp:950`/`970`).
                let spec = Self::key_spec(&key)?;
                let punch = key_is_punch(&key);
                let prep_feats = |p: bool| if p { 3 } else { 5 };
                let feats = match &self.last_press {
                    None => prep_feats(punch),
                    Some((pk, h, f, r, t)) => {
                        if key_is_punch(pk) != punch {
                            prep_feats(punch)
                        } else if punch {
                            if *h != spec.hand {
                                2
                            } else if *f != spec.finger {
                                1
                            } else {
                                0
                            }
                        } else if *h != spec.hand {
                            4
                        } else if *f != spec.finger {
                            3
                        } else {
                            // `distance=`/`direction=` (`motor.lisp:890`):
                            // peck features use the 2-degree / 45-degree
                            // tolerances, not exact polar equality.
                            let distance_same = (*r - spec.r).abs() < 2.0;
                            let delta = (*t - spec.theta).abs();
                            let direction_same = delta < std::f32::consts::FRAC_PI_4
                                || delta > 7.0 * std::f32::consts::PI / 4.0;
                            (if distance_same { 0 } else { 1 })
                                + (if direction_same { 0 } else { 1 })
                        }
                    }
                };
                self.last_press = Some((
                    key.to_uppercase(),
                    spec.hand.to_string(),
                    spec.finger.to_string(),
                    spec.r,
                    spec.theta,
                ));
                self.man_prep_busy = true;
                if self.cfg.zz_ext {
                    let style = if key_is_punch(&key) {
                        "PUNCH"
                    } else {
                        "PECK-RECOIL"
                    };
                    self.zz_last_style =
                        Some((style.to_string(), style.to_string(), spec.hand.to_string()));
                    let h = Self::touch_hand_idx(spec.hand).unwrap_or(1);
                    self.man_last_prep = Some(h);
                    self.zz_last_cmd[h] = style.to_string();
                    if self.cfg.zz_two_proc {
                        if let Some(h) = Self::touch_hand_idx(spec.hand) {
                            self.man_proc[h] = true;
                        }
                    } else {
                        self.man_proc[0] = true;
                        self.man_proc[1] = true;
                    }
                }
                let prep_s = feats as f32 * self.cfg.feat_prep_ms as f32 / 1000.0;
                let prep = (self.randomize_secs(prep_s) * 1000.0).round() as i64;
                let at = self.time_ms + prep;
                self.push(
                    at,
                    Prio::Num(0),
                    Out::Always,
                    Event::PreparationComplete { key, req_ms },
                );
            }
            Event::PreparationComplete { key, req_ms } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix()
                        ),
                    );
                }
                self.man_prep_busy = false;
                // `punch` (`motor.lisp:938`/`944`): exec = init + key-closure,
                // finish = init + burst + burst (no Fitts term).
                // `peck-recoil`: exec = init + max(burst, randomize(move)),
                // finish = exec + burst + max(burst, randomize(move)), with
                // move = max(burst, fitts(r)) (`motor.lisp:1019`/`1028`).
                let (exec_ms, finish_ms) = if key_is_punch(&key) {
                    (
                        self.cfg.init_ms + self.cfg.key_closure_ms,
                        self.cfg.init_ms + self.cfg.burst_ms + self.cfg.burst_ms,
                    )
                } else {
                    let spec = Self::key_spec(&key)?;
                    let burst_s = self.cfg.burst_ms as f32 / 1000.0;
                    let fit = (self.cfg.peck_fitts * (spec.r + 0.5).log2())
                        .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                    let move_s = burst_s.max(fit);
                    let exec_s =
                        self.cfg.init_ms as f32 / 1000.0 + burst_s.max(self.randomize_secs(move_s));
                    let finish_s = exec_s + burst_s + burst_s.max(self.randomize_secs(move_s));
                    // Lisp rounds seconds->ms at scheduling time.
                    (
                        (exec_s * 1000.0).round() as i64,
                        (finish_s * 1000.0).round() as i64,
                    )
                };
                if self.cfg.zz_ext {
                    // Queue for the hand's execution stage (`maybe-execute-movement`).
                    let (class, hand_name) = self
                        .zz_last_style
                        .clone()
                        .map(|x| (x.0, x.2))
                        .unwrap_or_else(|| ("PECK-RECOIL".to_string(), "RIGHT".to_string()));
                    let hand = Self::touch_hand_idx(&hand_name).unwrap_or(1);
                    self.man_exec_queue.push(ZzPending {
                        key,
                        req_ms,
                        hand,
                        both: false,
                        style: class,
                        exec_ms,
                        finish_ms,
                        kind: ZzKind::Press,
                    });
                    self.zz_maybe_execute();
                } else {
                    self.man_exec_busy = true;
                    let t = self.time_ms;
                    self.push(
                        t + self.cfg.init_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::InitiationComplete {
                            req_ms,
                            hand: 1,
                            both: false,
                            style: String::new(),
                        },
                    );
                    self.push(
                        t + exec_ms,
                        Prio::Num(0),
                        Out::Never,
                        Event::MotorNotify { key, req_ms },
                    );
                    self.push(
                        t + finish_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::FinishMovement {
                            req_ms,
                            hand: 1,
                            both: false,
                            style: String::new(),
                        },
                    );
                }
            }
            Event::InitiationComplete {
                req_ms,
                hand,
                both,
                style,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "INITIATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand(&style, hand, both)
                        ),
                    );
                }
                if self.cfg.zz_ext {
                    // ACT-R frees the hand of the *last prepared* movement
                    // (`motor-extension.lisp:1008`), which can differ from the
                    // movement whose initiation just completed.
                    if let Some(h) = self.man_last_prep {
                        if self.cfg.zz_two_proc {
                            self.man_proc[h] = false;
                        } else {
                            self.man_proc[0] = false;
                            self.man_proc[1] = false;
                        }
                    }
                }
            }
            Event::MotorNotify { key, req_ms } => {
                self.now_m(Prio::Num(0), Out::Always, true, Event::OutputKey { key });
                let _ = req_ms;
            }
            Event::OutputKey { key } => {
                let key = super::motor::canonical_output_key(&key);
                if self.sperling_collect && !key.eq_ignore_ascii_case("space") {
                    self.sperling_responses.push(key.clone());
                }
                self.last_output_key = Some((key.clone(), self.time_ms));
                self.fired_commands.push((
                    "output-key".to_string(),
                    self.model.name.clone(),
                    key.clone(),
                ));
                if show {
                    // The device reports its own key name (lowercase as
                    // specified in the keyboard table).
                    self.emit(
                        "KEYBOARD",
                        format!("output-key {} {}", self.model.name, key.to_lowercase()),
                    );
                }
                if self.tracker_task {
                    // `response` monitor (`simple.lisp:29`): prompt correct/
                    // wrong, then re-prompt `choose` 5 s later.
                    let prompt = if key == self.tracker_target {
                        "CORRECT"
                    } else {
                        "WRONG"
                    };
                    self.tracker_set_prompt(prompt);
                    self.push(self.time_ms + 5000, Prio::Num(0), Out::Never, Event::Choose);
                }
                if self.cfg.zz_ext {
                    self.zz_data
                        .push(("PRESSED".to_string(), self.time_ms, key.clone()));
                    // `keyboard-with-release` (`motor-extension.lisp:612`):
                    // a `release-key` signal `key-release-time` later.
                    self.push(
                        self.time_ms + self.cfg.zz_krt_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::ReleaseKey { key: key.clone() },
                    );
                }
                if self.clear_on_keypress {
                    // `clear-exp-window` from the task's output-key handler:
                    // drop the visicon and re-display. The re-encode of the
                    // attended location happens in `proc_display`
                    // (`update-attended-loc`), which also sees the cleared
                    // window because the visicon is dropped here first.
                    self.pending_clears.push(self.time_ms);
                    self.features.clear();
                    self.now_m(Prio::Num(-100), Out::Always, true, Event::ProcDisplay);
                }
            }
            Event::ReleaseKey { key } => {
                if show {
                    self.emit(
                        "KEYBOARD",
                        format!("release-key {} {}", self.model.name, key.to_lowercase()),
                    );
                }
                if self.cfg.zz_ext {
                    self.zz_data
                        .push(("RELEASED".to_string(), self.time_ms, key.clone()));
                }
            }
            Event::FinishMovement {
                req_ms,
                hand,
                both,
                style,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "FINISH-MOVEMENT {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand(&style, hand, both)
                        ),
                    );
                }
                if self.cfg.zz_ext {
                    if self.cfg.zz_two_exec && !both {
                        self.man_exec[hand] = false;
                    } else {
                        self.man_exec[0] = false;
                        self.man_exec[1] = false;
                    }
                    // `finish-movement-dual` (`motor-extension.lisp:1205`)
                    // frees the movement's fingers *before*
                    // `maybe-execute-movement` starts the next movement on the
                    // hand, so the free must precede the re-dispatch.
                    if both {
                        for h in 0..2 {
                            self.zz_hands[h].busy = [false; 5];
                        }
                    } else {
                        self.zz_hands[hand].busy = [false; 5];
                    }
                    self.zz_maybe_execute();
                } else {
                    self.man_exec[0] = false;
                    self.man_exec[1] = false;
                    self.man_exec_busy = false;
                }
                self.run_scripted_click();
            }
            Event::Warning { message } => {
                self.emit_warning_always(&message);
            }
            Event::Command { name, params } => {
                if !self.scheduled_command_known(&name) {
                    return Ok(());
                }
                if show {
                    let suffix = if params.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " {}",
                            params.iter().map(Value::show).collect::<Vec<_>>().join(" ")
                        )
                    };
                    self.emit("NONE", format!("{name}{suffix}"));
                }
                self.run_command(&name, params)?;
            }
        }
        Ok(())
    }

    /// `audio-event-ended` calls `update-sound-evt` and then mods the
    /// `AURAL-LOCATION` buffer (`audio.lisp:1077`). The buffer holds a
    /// `{source}-{k}` copy, so only its slots change.
    fn update_audio_location_buffer(&mut self, idx: usize) {
        let s = self.sounds[idx].clone();
        let event = audio_event_chunk(&s, self.time_ms, self.cfg.audio_ms_times);
        let Some(buf) = self.buffers.get_mut("AURAL-LOCATION") else {
            return;
        };
        let Some(chunk) = buf.chunk.as_mut() else {
            return;
        };
        for (slot, value) in event.slots {
            match chunk.slots.iter_mut().find(|(k, _)| *k == slot) {
                Some((_, old)) => *old = value,
                None => chunk.slots.push((slot, value)),
            }
        }
        // `audio-event-ended`'s mod does not disqualify the auto-stuffed copy
        // from overstuff/unstuff (`stuff-sound-buffer`, `audio.lisp:716`).
    }
}
