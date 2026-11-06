//! Vision, audio, and EMMA.
use super::*;
/// `decode-string` (`dispatcher.lisp:911`): a value wrapped in single quotes
/// is a literal string; otherwise it names a Lisp symbol.
fn decode_audio_string(s: &str) -> String {
    if s.len() > 1 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

impl Engine {
    /// `new-tone-sound` (`audio.lisp:778`): create a tone sound event. Names
    /// come from the two independent counters (`TONE{n}` / `AUDIO-EVENT{n}`),
    /// and `tone-sound-evt` sets `delay`/`recode`/`pitch` at construction
    /// (`audio.lisp:649`).
    pub(super) fn add_sound(&mut self, freq: i64, duration: i64, onset: i64) -> usize {
        let n = self.tone_seq;
        self.tone_seq += 1;
        let en = self.audio_events;
        self.audio_events += 1;
        // `(> content 1500)` high, `(< content 900)` low, else middle.
        let pitch = if freq > 1500 {
            "HIGH"
        } else if freq < 900 {
            "LOW"
        } else {
            "MIDDLE"
        };
        self.sounds.push(Sound {
            sname: format!("TONE{n}"),
            ename: format!("AUDIO-EVENT{en}"),
            kind: "TONE".to_string(),
            content: Value::Int(freq),
            location: "EXTERNAL".to_string(),
            onset,
            offset: Some(onset + duration),
            duration: Some(duration),
            delay: 50,
            recode: 285,
            pitch: pitch.to_string(),
            attended: SoundAttended::Nil,
            other_features: Vec::new(),
        });
        let idx = self.sounds.len() - 1;
        self.sync_audio_chunks(idx);
        self.schedule_sound_detection(idx);
        idx
    }

    /// `new-sound-event :around` (`audio.lisp:733`): schedule
    /// `stuff-sound-buffer` at `detect-at-time` when the sound is created.
    /// Scheduling at creation, not at onset, preserves the queue order for a
    /// detection that lands at the same time as another sound's `new-sound`.
    fn schedule_sound_detection(&mut self, idx: usize) {
        let s = &self.sounds[idx];
        let at = s.onset + s.delay;
        self.push_m(
            at,
            Prio::Num(0),
            Out::Always,
            true,
            Event::StuffSound { idx },
        );
    }

    /// `new-digit-sound` (`audio.lisp:760`): a `DIGIT` sound whose `content`
    /// is the digit itself. The audio module's defaults are 600 ms duration,
    /// 300 ms detect delay, and 500 ms recode delay (`audio.lisp:540`).
    ///
    /// Lisp's `sound-event` `:after` method computes `offset` before the
    /// `digit-sound-evt` `:after` method installs `duration`, so a digit's
    /// offset stays at its onset even though its duration is 600 ms.
    pub(super) fn new_digit_sound(&mut self, digit: i64, onset: i64) {
        let n = self.digit_seq;
        self.digit_seq += 1;
        let en = self.audio_events;
        self.audio_events += 1;
        self.sounds.push(Sound {
            sname: format!("DIGIT{n}"),
            ename: format!("AUDIO-EVENT{en}"),
            kind: "DIGIT".to_string(),
            content: Value::Int(digit),
            location: "EXTERNAL".to_string(),
            onset,
            offset: Some(onset),
            duration: Some(600),
            delay: 300,
            recode: 500,
            pitch: "NIL".to_string(),
            attended: SoundAttended::Nil,
            other_features: Vec::new(),
        });
        let idx = self.sounds.len() - 1;
        self.sync_audio_chunks(idx);
        self.schedule_sound_detection(idx);
        self.push(onset, Prio::Num(0), Out::Medium, Event::ToneSound);
    }

    /// `new-word-sound` (`audio.lisp:901`): a `WORD` sound whose `content`
    /// is the word itself.  Duration comes from the speech module's
    /// articulation time; `word-sound-evt` recodes at
    /// `max(duration/2, duration-150)` and detects after the digit delay.
    pub(super) fn new_word_sound(&mut self, word: String, onset: i64, location: String) {
        let n = self.word_seq;
        self.word_seq += 1;
        let en = self.audio_events;
        self.audio_events += 1;
        let duration = self.articulation_time_ms(&word);
        let recode = (duration / 2).max(duration - 150);
        self.sounds.push(Sound {
            sname: format!("WORD{n}"),
            ename: format!("AUDIO-EVENT{en}"),
            kind: "WORD".to_string(),
            content: Value::Str(word),
            location: location.to_uppercase(),
            onset,
            offset: Some(onset + duration),
            duration: Some(duration),
            delay: 300,
            recode,
            pitch: "MIDDLE".to_string(),
            attended: SoundAttended::Nil,
            other_features: Vec::new(),
        });
        let idx = self.sounds.len() - 1;
        self.sync_audio_chunks(idx);
        self.schedule_sound_detection(idx);
        self.push(onset, Prio::Num(0), Out::Medium, Event::ToneSound);
    }

    /// `new-ongoing-sound` (`audio.lisp:840`): a custom sound with no offset
    /// until `end-ongoing-sound`. Returns the command's id.
    pub fn new_ongoing_sound_rpc(
        &mut self,
        content: String,
        delay_ms: i64,
        recode_ms: i64,
        onset: Option<i64>,
        location: String,
        kind: String,
        other_features: &[crate::sexpr::Sexp],
    ) -> Result<i64, EngineError> {
        let mut features = Vec::new();
        for feature in other_features {
            let Sexp::List(items) = feature else {
                return Err(bad("other sound feature is not a list".to_string()));
            };
            if items.len() < 3 {
                return Err(bad("other sound feature needs three items".to_string()));
            }
            let scope = match items[0]
                .as_sym()
                .unwrap_or("")
                .trim_start_matches(':')
                .to_uppercase()
                .as_str()
            {
                "EVT" => AudioFeatureScope::Evt,
                "SOUND" => AudioFeatureScope::Sound,
                "BOTH" => AudioFeatureScope::Both,
                _ => return Err(bad("bad sound feature scope".to_string())),
            };
            let slot = items[1]
                .as_sym()
                .ok_or_else(|| bad("bad sound feature slot".to_string()))?
                .to_uppercase();
            self.extend_possible_slots(&slot);
            let value = match Value::from_sexp(&items[2])? {
                Value::Str(s) => Value::Str(decode_audio_string(&s)),
                other => other,
            };
            features.push(AudioFeature { scope, slot, value });
        }
        let n = self.sound_seq;
        self.sound_seq += 1;
        let en = self.audio_events;
        self.audio_events += 1;
        let at = onset.unwrap_or(self.time_ms);
        self.sounds.push(Sound {
            sname: format!("SOUND{n}"),
            ename: format!("AUDIO-EVENT{en}"),
            kind: kind.to_uppercase(),
            content: Value::Str(decode_audio_string(&content)),
            location: location.to_uppercase(),
            onset: at,
            offset: None,
            duration: None,
            delay: delay_ms,
            recode: recode_ms,
            pitch: "MIDDLE".to_string(),
            attended: SoundAttended::Nil,
            other_features: features,
        });
        let idx = self.sounds.len() - 1;
        self.sync_audio_chunks(idx);
        self.schedule_sound_detection(idx);
        self.ongoing_seq += 1;
        let id = self.ongoing_seq as i64;
        self.ongoing_sounds.insert(id as u64, idx);
        self.push(at, Prio::Num(0), Out::Medium, Event::ToneSound);
        Ok(id)
    }

    /// `end-ongoing-sound` (`audio.lisp:877`): stop an ongoing sound at the
    /// current model time and schedule its high-detail end check.
    pub fn end_ongoing_sound_rpc(&mut self, id: i64) -> Result<(), EngineError> {
        self.end_ongoing_sound(id as u64)
    }

    pub(super) fn end_ongoing_sound(&mut self, id: u64) -> Result<(), EngineError> {
        let Some(idx) = self.ongoing_sounds.remove(&id) else {
            return Ok(());
        };
        let now = self.time_ms;
        let sound = &mut self.sounds[idx];
        sound.offset = Some(now);
        sound.duration = Some(now - sound.onset);
        let evt = sound
            .ename
            .trim_start_matches("AUDIO-EVENT")
            .parse::<u64>()
            .map_err(|_| bad("bad audio-event name".to_string()))?;
        self.push_m(
            now,
            Prio::Num(9),
            Out::Always,
            true,
            Event::AudioEventEnded { evt },
        );
        Ok(())
    }

    /// `update-sound-evt` (`audio.lisp:922`): refresh the source audio chunks
    /// from the live sound state. The event chunk only gains `offset` /
    /// `duration` once the sound is finished.
    pub(super) fn sync_audio_chunks(&mut self, idx: usize) {
        let s = self.sounds[idx].clone();
        let event = audio_event_chunk(&s, self.time_ms, self.cfg.audio_ms_times);
        self.named.insert(s.ename.clone(), event);
        self.sync_sound_chunk(idx);
    }

    pub(super) fn sync_sound_chunk(&mut self, idx: usize) {
        let sound = sound_chunk(&self.sounds[idx]);
        self.named
            .insert(sound.name.clone().unwrap_or_default(), sound);
    }

    /// Device request routing for the visual buffer: only
    /// move-attention (plus finst assignment later).
    pub(super) fn visual_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        // `pm-module-request` for the visual buffer (`vision.lisp:3240`): a
        // request releases the vision module's proc-display lock and schedules
        // the deferred `unlock-vision`, which re-runs a blocked proc-display.
        if self.visual_lock {
            self.visual_lock = false;
            // `:maintenance t` (`vision.lisp:3244`): a maintenance event does
            // not mark buffers changed, so it must not wake conflict resolution.
            self.push_m(
                self.time_ms,
                Prio::Min,
                Out::Never,
                true,
                Event::UnlockVision,
            );
        }
        let (slots, ctype) = match &spec {
            NewChunk::Spec { ctype, slots, .. } => (slots.clone(), ctype.clone()),
            _ => return Err(unimpl("non-spec visual request")),
        };
        let mut cmd: Option<String> = None;
        let mut screen_pos: Option<String> = None;
        let mut scale: Option<String> = None;
        for (s, v) in &slots {
            match (s.as_str(), v) {
                ("CMD", Value::Sym(c)) => cmd = Some(c.clone()),
                ("SCREEN-POS", Value::Sym(n)) => screen_pos = Some(n.clone()),
                ("SCALE", Value::Sym(n)) => scale = Some(n.clone()),
                got => {
                    return Err(unimpl(format!("visual request slot {got:?}")));
                }
            }
        }
        // A `+visual>` request's command is its chunk-type (`isa
        // move-attention` / `isa start-tracking`), with a `cmd` slot
        // overriding it.
        let command = cmd
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| ctype.clone());
        match command.as_str() {
            "MOVE-ATTENTION" => {
                let loc = screen_pos
                    .ok_or_else(|| bad("move-attention without screen-pos".to_string()))?;
                if !self.loc_known(&loc) {
                    return Err(bad(format!("screen-pos {loc} is not a visual location")));
                }
                let _ = scale;
                self.now(Prio::Num(0), Out::Always, Event::MoveAttention { loc });
                Ok(())
            }
            // `+visual> cmd clear`: direct clear event (output low);
            // the buffer itself is cleared by the request-rule implicit
            // ClearBuffer already queued at firing.
            "CLEAR" => {
                // `pm-module-request` schedules `clear`
                // (`vision.lisp:3252`), whose `pm-module` method sets `prep`
                // busy for `feat-prep-time` (`general-pm.lisp:333`); the
                // matching `change-state` frees it.
                self.visual_prep_busy = true;
                self.now(Prio::Num(0), Out::Low, Event::VisualClear);
                Ok(())
            }
            // `start-tracking` (`vision.lisp:2665`): begin tracking the
            // currently attended object; `update-tracking-mth` then stuffs
            // the vision buffers. Scheduled so the request rule's implicit
            // clear-buffer lands first.
            "START-TRACKING" => {
                self.now(Prio::Num(0), Out::Always, Event::StartTracking);
                Ok(())
            }
            got => Err(unimpl(format!("visual command {got:?}"))),
        }
    }

    /// A visual location's `(width, height)` (0,0 when absent).
    pub(super) fn loc_wh(&self, name: &str) -> (i64, i64) {
        if let Some(&idx) = self.vl_copies.get(name) {
            return (self.features[idx].width, self.features[idx].height);
        }
        if let Some(f) = self.features.iter().find(|f| f.name == name) {
            return (f.width, f.height);
        }
        // A manual target may be an imaginal/goal chunk carrying width/height.
        if let Some(c) = self.find_chunk_by_name(name) {
            let slot = |k: &str| {
                c.slots
                    .iter()
                    .find(|(s, _)| s == k)
                    .and_then(|(_, v)| match v {
                        Value::Int(i) => Some(*i),
                        Value::Num(x) => Some(*x as i64),
                        _ => None,
                    })
            };
            return (slot("WIDTH").unwrap_or(0), slot("HEIGHT").unwrap_or(0));
        }
        (0, 0)
    }

    /// A chunk-valued slot's `(screen-x, screen-y, name)` (`chunk-to-visual-position`).
    pub(super) fn chunk_xy_name(&self, v: &Value) -> Option<(i64, i64, String)> {
        let name = match v {
            Value::Sym(s) | Value::Str(s) => s.clone(),
            _ => return None,
        };
        if self.vl_copies.contains_key(&name) {
            if let Some(&(x, y)) = self.vl_xy.get(&name) {
                return Some((x, y, name));
            }
        }
        if let Some(idx) = self.features.iter().position(|f| f.name == name) {
            let f = &self.features[idx];
            return Some((f.x, f.y, name));
        }
        let chunk = self
            .named
            .get(&name)
            .cloned()
            .or_else(|| {
                self.dm
                    .iter()
                    .find(|c| c.name.as_deref() == Some(&name))
                    .cloned()
            })
            .or_else(|| {
                self.buffers.values().find_map(|b| {
                    b.chunk
                        .as_ref()
                        .filter(|c| c.name.as_deref() == Some(&name))
                        .cloned()
                })
            });
        let c = chunk?;
        let slot = |k: &str| -> Option<i64> {
            c.slots
                .iter()
                .find(|(s, _)| s == k)
                .and_then(|(_, v)| match v {
                    Value::Int(i) => Some(*i),
                    Value::Num(x) => Some(*x as i64),
                    _ => None,
                })
        };
        Some((slot("SCREEN-X")?, slot("SCREEN-Y")?, name))
    }

    pub(super) fn loc_known(&self, loc: &str) -> bool {
        let is_visual_location = |c: &ChunkInst| c.ctype.eq_ignore_ascii_case("VISUAL-LOCATION");
        self.vl_copies.contains_key(loc)
            || self.features.iter().any(|f| f.name == loc)
            || self
                .buffers
                .values()
                .any(|b| b.chunk.as_ref().and_then(|c| c.name.as_deref()) == Some(loc))
            || self.named.get(loc).is_some_and(is_visual_location)
            || self
                .dm
                .iter()
                .any(|c| c.name.as_deref() == Some(loc) && is_visual_location(c))
    }

    /// A location name (feature base, request copy, or buffer chunk) to
    /// its feature index.
    /// The xyz vector of a location chunk (`xyz-loc`, `vision.lisp:1839`):
    /// `(screen-x screen-y distance)`, the distance being
    /// `viewing-distance × pixels-per-inch` (18 × 60 = 1080).
    pub(super) fn emma_loc_xyz(&self, loc: &str) -> (f32, f32, f32) {
        let (x, y) = self
            .vl_xy
            .get(loc)
            .copied()
            .or_else(|| {
                self.features
                    .iter()
                    .find(|f| f.name == loc)
                    .map(|f| (f.x, f.y))
            })
            .unwrap_or((0, 0));
        (x as f32, y as f32, 1080.0)
    }

    pub(super) fn resolve_loc(&self, loc: &str) -> Result<usize, EngineError> {
        if let Some(&idx) = self.vl_copies.get(loc) {
            return Ok(idx);
        }
        if let Some(idx) = self.features.iter().position(|f| f.name == loc) {
            return Ok(idx);
        }
        Err(bad(format!("screen-pos {loc} is not a visual location")))
    }

    /// The current marker's value for a visual-location slot. `current` in a
    /// visual-location request means the last attended location
    /// (`find-current-locs-with-spec`, `vision.lisp:2076`).
    fn current_loc_slot(&self, slot: &str) -> Option<Value> {
        let idx = self
            .last_attention
            .as_ref()
            .and_then(|loc| self.resolve_loc(loc).ok())?;
        self.features[idx]
            .loc_slots()
            .into_iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(slot))
            .map(|(_, value)| value)
    }

    /// proc-display: move due device texts into the visicon, stuff the
    /// visual-location buffer if empty, and note the update.
    // ------------------------------------------------------------------
    // EMMA (`extras/emma/emma.lisp`): eye-movement timing and scheduling.
    // ------------------------------------------------------------------

    /// `emma-attention-hook` (`emma.lisp:502`): `move-attention` calls this
    /// instead of the plain encoding schedule when `:emma` is enabled.
    pub(super) fn emma_attention_hook(&mut self, end_loc: (f32, f32, f32)) {
        // `next-destination` (`:546`): the current marker when the eye is
        // free, otherwise where it will land.
        let start = if !self.emma_exec_s {
            self.emma_eye
        } else {
            self.emma_next_loc.unwrap_or(self.emma_eye)
        };
        let return_obj = self.emma_obj_at(end_loc);
        let sacc = self.emma_compute_saccade(start, end_loc);
        let dur = self.emma_recog_time(sacc.r);
        self.emma_shift_target = return_obj;
        self.emma_final_shift = end_loc;
        self.emma_initiate_eye_move(dur, sacc);
    }

    /// `get-obj-at-location` (`vision.lisp:2485`): the object at `loc`, or
    /// none. Building one consumes an object number (`TEXT{n}`), matching
    /// the ref's TEXT0/TEXT1 then TEXT2 from the encoding.
    pub(super) fn emma_obj_at(&mut self, loc: (f32, f32, f32)) -> Option<String> {
        // `within-move` (`vision.lisp:2367`): features within the movement
        // allowance of the location (default `:visual-movement-tolerance`).
        let idx = self.features.iter().position(|f| {
            let fl = (f.x as f32, f.y as f32, loc.2);
            angle_between(loc, fl) <= 1.0
        })?;
        let kind = self.features[idx].kind.clone();
        let k = self.obj_built.entry(kind.clone()).or_insert(0);
        let name = format!("{kind}{k}");
        *k += 1;
        Some(name)
    }

    /// `compute-saccade-r-theta` (`emma.lisp:675`).
    pub(super) fn emma_compute_saccade(
        &mut self,
        start: (f32, f32, f32),
        end: (f32, f32, f32),
    ) -> EmmaSacc {
        let stddev = 0.1 * vec_dist(start, end);
        let ex = lisp_round(add_gaussian_noise(
            &mut self.rng.borrow_mut(),
            end.0,
            stddev,
        )) as f32;
        let ey = lisp_round(add_gaussian_noise(
            &mut self.rng.borrow_mut(),
            end.1,
            stddev,
        )) as f32;
        let new_loc = (ex, ey, end.2);
        let r = angle_between(start, new_loc);
        let theta = (start.1 - end.1).atan2(end.0 - start.0);
        let fprep_time = self.emma_compute_prep_time(r, theta);
        let exec_time = self.emma_compute_exec_time(r);
        EmmaSacc {
            r,
            theta,
            trgt: None,
            new_loc,
            fprep_time,
            exec_time,
        }
    }

    /// `compute-prep-time` (`general-pm.lisp:670`): 3 features when there is
    /// no previous movement, else the differing r/theta features
    /// (`feat-differences`, `emma.lisp:482`; `distance=`/`direction=`,
    /// `motor.lisp:888`/`:894`).
    pub(super) fn emma_compute_prep_time(&self, r: f32, theta: f32) -> f64 {
        let base = self.cfg.emma_feat_prep as f64;
        match &self.emma_last_prep {
            None => base * 3.0,
            Some(prev) => {
                let mut n = 0.0;
                if !(2.0 > (r - prev.r).abs()) {
                    n += 1.0;
                }
                let delta = (theta - prev.theta).abs();
                let same = (std::f32::consts::PI / 4.0) > delta
                    || delta > (7.0 / 4.0 * std::f32::consts::PI);
                if !same {
                    n += 1.0;
                }
                base * n
            }
        }
    }

    /// `compute-exec-time` (`emma.lisp:461`): `init + r·rate + base`.
    pub(super) fn emma_compute_exec_time(&self, r: f32) -> f64 {
        (self.cfg.emma_init_time + r * self.cfg.emma_sacc_rate + self.cfg.emma_base_exe) as f64
    }

    /// `recog-time` (`emma.lisp:696`): `K·(−ln f)·e^(r·k)`.
    pub(super) fn emma_recog_time(&self, r: f32) -> f64 {
        let freq = self.cfg.emma_obj_freq;
        (self.cfg.emma_enc_factor * (-freq.ln()) * (r * self.cfg.emma_enc_exp).exp()) as f64
    }

    /// `total-time` (`emma.lisp:494`): prep + exec.
    pub(super) fn emma_total_time(sacc: &EmmaSacc) -> f64 {
        sacc.fprep_time + sacc.exec_time
    }

    /// `initiate-eye-move` (`emma.lisp:566`).
    pub(super) fn emma_initiate_eye_move(&mut self, recog_time: f64, mut sacc: EmmaSacc) {
        sacc.trgt = self.emma_shift_target.clone();
        self.emma_shift_start_ms = self.time_ms;
        self.emma_prep_s = true;
        self.emma_last_prep = Some(sacc.clone());
        let at = self.time_ms + (sacc.fprep_time.max(0.0) * 1000.0).round() as i64;
        self.push(
            at,
            Prio::Num(0),
            Out::Medium,
            Event::EmmaPrepComplete {
                trgt: sacc.trgt.clone(),
            },
        );
        if recog_time <= Self::emma_total_time(&sacc) {
            self.emma_schedule_encoding_complete(recog_time);
            self.emma_encoded = true;
        } else {
            self.emma_encoded = false;
        }
    }

    /// `schedule-encoding-complete` (`vision.lisp:3043`): the plain VISION
    /// encoding at the last attention location.
    pub(super) fn emma_schedule_encoding_complete(&mut self, delay: f64) {
        let Some(loc) = self.last_attention.clone() else {
            return;
        };
        let at = self.time_ms + (delay.max(0.0) * 1000.0).round() as i64;
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

    /// `preparation-complete` + `maybe-execute-movement` (`general-pm.lisp`).
    pub(super) fn emma_preparation_complete(&mut self) {
        self.emma_prep_s = false;
        let Some(sacc) = self.emma_last_prep.clone() else {
            return;
        };
        if !self.emma_exec_s {
            self.emma_perform_movement(sacc);
        }
    }

    /// EMMA's `perform-movement` override (`emma.lisp:653`).
    pub(super) fn emma_perform_movement(&mut self, sacc: EmmaSacc) {
        self.emma_exec_s = true;
        self.emma_next_loc = Some(sacc.new_loc);
        let init = sacc.exec_time.max(0.0).min(self.cfg.emma_init_time as f64);
        let init_at = self.time_ms + (init * 1000.0).round() as i64;
        self.push(init_at, Prio::Num(0), Out::Medium, Event::EmmaInitComplete);
        let move_at = self.time_ms + (sacc.exec_time.max(0.0) * 1000.0).round() as i64;
        self.push(
            move_at,
            Prio::Num(0),
            Out::Medium,
            Event::EmmaEyeMoveComplete { sacc },
        );
    }

    /// `complete-eye-move` (`emma.lisp:620`): land, then re-fixate when the
    /// object was not encoded before the movement finished.
    pub(super) fn emma_complete_eye_move(&mut self, sacc: EmmaSacc) {
        self.emma_eye = sacc.new_loc;
        if !self.emma_encoded {
            let new_target = self.emma_obj_at(self.emma_final_shift);
            let start = self.emma_eye;
            let end = self.emma_final_shift;
            let sacc2 = self.emma_compute_saccade(start, end);
            let elapsed = (self.time_ms - self.emma_shift_start_ms) as f64 / 1000.0;
            let frac = if self.emma_shift_duration != 0.0 {
                elapsed / self.emma_shift_duration
            } else {
                0.0
            };
            let new_duration = self.emma_recog_time(sacc2.r) * (1.0 - frac);
            self.emma_shift_target = new_target;
            self.emma_shift_start_ms = self.time_ms;
            self.emma_shift_duration = new_duration;
            self.emma_initiate_eye_move(new_duration, sacc2);
        }
        self.emma_exec_s = false;
    }

    /// A window-text visicon feature (`build-string-feats` + the device's
    /// `color`/`kind`/`text`), shared by `proc_display` and the pending
    /// (`printed-visicon`) view so the two agree.
    pub(super) fn text_feature(
        &self,
        name: String,
        text: &str,
        sx: i64,
        sy: i64,
        tstamp: i64,
    ) -> Feature {
        let width = 7 * text.chars().count() as i64;
        Feature {
            name,
            kind: "TEXT".to_string(),
            value: text.to_string(),
            x: sx,
            y: sy,
            width,
            height: 10,
            extra: Vec::new(),
            spec: vec![
                ("SCREEN-Y".to_string(), Value::Int(sy)),
                ("HEIGHT".to_string(), Value::Int(10)),
                ("VALUE".to_string(), Value::Str(text.to_string())),
                ("WIDTH".to_string(), Value::Int(width)),
                ("SCREEN-X".to_string(), Value::Int(sx)),
                ("COLOR".to_string(), Value::Sym("BLACK".to_string())),
                ("KIND".to_string(), Value::Sym("TEXT".to_string())),
                ("TEXT".to_string(), Value::Sym("T".to_string())),
            ],
            spec_pre: Vec::new(),
            tstamp_ms: tstamp,
            new_p: true,
            finst_ms: None,
        }
    }

    pub(super) fn proc_display(&mut self) -> Result<(), EngineError> {
        let now = self.time_ms;
        // `internal-proc-display` (`vision.lisp:1577`): while the vision module
        // is locked the display is skipped and flagged, to be re-run by
        // `unlock-vision`.
        if self.visual_lock {
            self.blocked_proc_display = true;
            return Ok(());
        }
        // A task `print-visicon` inside a run is pure output; it is replayed
        // as an event at the model time the task called it (see
        // `arm_device_script`), before this wave's `visicon-update`.
        // This *is* the display wave for `now`, so window ops applied inside it
        // (the scripted buttons/lines below, and the `add-text` a button's
        // label goes through) must not queue another one — the ref has a single
        // `visicon-update` for a whole `build-display`.
        self.display_queued_ms = Some(now);
        // A task's setup draws happen before the `add-text` calls at the same
        // instant (`pcomp-issues` picks the two trial numbers first).
        let mut due_rng = Vec::new();
        self.pending_rng.retain(|r| {
            if r.at_ms <= now {
                due_rng.push(r.clone());
                false
            } else {
                true
            }
        });
        for r in due_rng {
            for _ in 0..r.count {
                self.rng.borrow_mut().random_int(r.limit);
            }
        }
        // `process-display` runs `check-finsts` (`vision.lisp:1796`): finsts older
        // than `:visual-finst-span` are dropped before the display is searched.
        self.check_visual_finsts();
        // `process-display` (`vision.lisp:1634`): drain the queued
        // `visicon-updates` into the visicon. `:delete-all` clears everything;
        // the others add, remove or modify one feature.
        for u in std::mem::take(&mut self.visicon_pending) {
            match u {
                VisPending::Add(mut f) => {
                    // `process-display` stamps the feature at *processing*
                    // time (`vision.lisp:1640`), which `newest_feature` uses.
                    f.tstamp_ms = now;
                    self.features.push(f);
                }
                VisPending::Delete(name) => self.features.retain(|f| f.name != name),
                VisPending::Update(name, mods) => {
                    if let Some(f) = self.features.iter_mut().find(|f| f.name == name) {
                        f.tstamp_ms = now;
                        for (k, v) in mods {
                            f.set_slot(&k, v);
                        }
                    }
                }
                VisPending::DeleteAll => self.features.clear(),
            }
        }
        let mut due = Vec::new();
        self.pending_texts.retain(|t| {
            if t.at_ms <= now {
                due.push(t.clone());
                false
            } else {
                true
            }
        });
        let mut cleared = false;
        self.pending_clears.retain(|c| {
            if *c <= now {
                cleared = true;
                false
            } else {
                true
            }
        });
        if cleared {
            // `:delete-all` collects attended features for the rebuilt display
            // (`transfer-finsts`, `vision.lisp:1656`).
            self.collect_finst_transfers();
            self.features.clear();
        }
        // Window-item moves: re-apply the captured position change to the
        // visicon feature carrying that text value (the model's `!eval!`
        // schedules `move-it`, which the engine replays as a no-op; the
        // captured op supplies the coordinates).
        let mut moved = Vec::new();
        self.pending_modifies.retain(|m| {
            if m.at_ms <= now {
                moved.push(m.clone());
                false
            } else {
                true
            }
        });
        for m in &moved {
            if let Some(f) = self.features.iter_mut().find(|f| f.value == m.text) {
                f.x = m.x + 305;
                f.y = m.y + 306;
                f.tstamp_ms = now;
            }
        }
        // `modify-line-for-exp-window` (e.g. the `bst` task's
        // `update-current-line`): redraw the line whose *old* endpoints match,
        // keeping its identity — ACT-R's `internal-modify-visicon-features`
        // queues an `:update`, so the chunk (and its finst) survives.
        let mut due_line_mods: Vec<DeviceLineModify> = Vec::new();
        self.pending_line_modifies.retain(|m| {
            if m.at_ms <= now {
                due_line_mods.push(m.clone());
                false
            } else {
                true
            }
        });
        for m in &due_line_mods {
            let (ox, oy) = self.window_origin;
            let matches = |f: &Feature| {
                f.kind == "LINE"
                    && f.extra
                        .iter()
                        .any(|(k, v)| k == "END1-X" && *v == Value::Int(ox + m.x1))
                    && f.extra
                        .iter()
                        .any(|(k, v)| k == "END1-Y" && *v == Value::Int(oy + m.y1))
                    && f.extra
                        .iter()
                        .any(|(k, v)| k == "END2-X" && *v == Value::Int(ox + m.x2))
                    && f.extra
                        .iter()
                        .any(|(k, v)| k == "END2-Y" && *v == Value::Int(oy + m.y2))
            };
            if let Some(f) = self.features.iter_mut().find(|f| matches(f)) {
                let color = f
                    .extra
                    .iter()
                    .find(|(k, _)| k == "COLOR")
                    .and_then(|(_, v)| match v {
                        Value::Sym(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "BLACK".to_string());
                let origin = (ox, oy);
                debug::set_line_geometry(f, (m.nx1, m.ny1), (m.nx2, m.ny2), origin, &color);
                f.tstamp_ms = now;
            }
        }
        // `remove-items-from-exp-window` of a line item: drop the line whose
        // endpoints match.
        let mut due_line_removes: Vec<DeviceLineRemove> = Vec::new();
        self.pending_line_removes.retain(|r| {
            if r.at_ms <= now {
                due_line_removes.push(r.clone());
                false
            } else {
                true
            }
        });
        for r in &due_line_removes {
            let (ox, oy) = self.window_origin;
            self.features.retain(|f| {
                !(f.kind == "LINE"
                    && f.extra
                        .iter()
                        .any(|(k, v)| k == "END1-X" && *v == Value::Int(ox + r.x1))
                    && f.extra
                        .iter()
                        .any(|(k, v)| k == "END1-Y" && *v == Value::Int(oy + r.y1))
                    && f.extra
                        .iter()
                        .any(|(k, v)| k == "END2-X" && *v == Value::Int(ox + r.x2))
                    && f.extra
                        .iter()
                        .any(|(k, v)| k == "END2-Y" && *v == Value::Int(oy + r.y2)))
            });
        }
        // `remove-items-from-exp-window`: drop the item's visicon feature.
        // Like a clear it re-encodes the attended location
        // (`update-attended-loc`), which is how the ref's removed letter
        // produces `Encoding-complete`/`No visual-object found` 85 ms later.
        let mut due_removes: Vec<DeviceRemove> = Vec::new();
        self.pending_removes.retain(|r| {
            if r.at_ms <= now {
                due_removes.push(r.clone());
                false
            } else {
                true
            }
        });
        for r in &due_removes {
            self.features.retain(|f| f.value != r.text);
        }
        let removed = !due_removes.is_empty();
        // `update-attended-loc` (`vision.lisp:1870`): re-encode when the
        // attended object is gone from the visicon. The attended name is a
        // *copy* (`VISUAL-LOCATION0-1`), so it is resolved through
        // `chunk-visicon-entry` first; a new text elsewhere leaves the old
        // location in place, so adding texts must not trigger this
        // (paired-static: the old location stays). EMMA re-invokes its hook
        // instead (a fresh saccade); the base module re-schedules
        // `encoding-complete` after the attention latency.
        let attended_gone = match self.last_attention.clone() {
            Some(loc) => {
                let base = self
                    .vl_copied_from
                    .get(&loc)
                    .cloned()
                    .unwrap_or(loc.clone());
                !self.features.iter().any(|f| f.name == base)
            }
            None => false,
        };
        let _ = (cleared, removed);
        // `update-attended-loc` returns early when a movement already occupies
        // the visual module (Sperling's 0.941 clear is processed at 0.975,
        // after the new move-attention has set exec BUSY, so only that
        // movement's requested encode lands at 1.060).
        if attended_gone && !self.visual_busy {
            if let Some(loc) = self.last_attention.clone() {
                if self.cfg.emma {
                    let end = self.emma_loc_xyz(&loc);
                    self.emma_attention_hook(end);
                } else {
                    // The re-encode occupies the visual module, so
                    // `?visual> state free` fails until it lands
                    // (`update-attended-loc` -> `encode-visual-object`).
                    self.visual_busy = true;
                    let at = self.time_ms + self.randomize_ms(self.cfg.attention_ms);
                    self.push(
                        at,
                        Prio::Num(0),
                        Out::Medium,
                        Event::EncodingComplete {
                            loc,
                            requested: false,
                        },
                    );
                }
            }
        }
        // The visible RPM window returns its subviews column-major, so the
        // `VISUAL-LOCATION{n}` names follow `(screen-x, screen-y)` order
        // (verified against `sperling`: the input order `V N T Z / C R Y K /
        // W J G F` names `V C W N R J T Y G Z K F` as `0..11`). An invisible
        // virtual window keeps the task's add order instead
        // (`nearest-vis-loc`'s `x, y, z`).
        if self.device_window_visible {
            due.sort_by_key(|t| (t.x, t.y));
        }
        for t in due {
            let n = self.feat_seq;
            self.feat_seq += 1;
            // `add-text-to-exp-window` places a text item relative to the
            // window by its top-left, and the visicon uses the item's
            // centre: `screen-x = window-x + text-x + width/2`,
            // `screen-y = window-y + text-y + height/2` (verified against
            // SBCL). The test-model windows use the default origin
            // (300, 300) and the font is 7 px/char, 10 px tall, so the
            // offsets are `305 + (width-7)/2` and `306`.
            let width = 7 * t.text.chars().count() as i64;
            let sx = t.x + 305 + (width - 7) / 2;
            let sy = t.y + 306;
            self.features.push(self.text_feature(
                format!("VISUAL-LOCATION{n}"),
                &t.text,
                sx,
                sy,
                now,
            ));
        }
        // Scripted buttons and lines (`add-button-to-exp-window` /
        // `add-line-to-exp-window`): build their features through the same
        // helpers the RPC device API uses, so a scripted replay and a driven
        // run produce the same visicon. They are merged by `at_ms` because the
        // task interleaves them with texts and with each other.
        let mut due_buttons = Vec::new();
        self.pending_buttons.retain(|b| {
            if b.at_ms <= now {
                due_buttons.push(b.clone());
                false
            } else {
                true
            }
        });
        let mut due_lines = Vec::new();
        self.pending_lines.retain(|l| {
            if l.at_ms <= now {
                due_lines.push(l.clone());
                false
            } else {
                true
            }
        });
        let (mut bi, mut li) = (0usize, 0usize);
        while bi < due_buttons.len() || li < due_lines.len() {
            let take_button = match (due_buttons.get(bi), due_lines.get(li)) {
                (Some(b), Some(l)) => b.at_ms <= l.at_ms,
                (Some(_), None) => true,
                _ => false,
            };
            if take_button {
                let b = &due_buttons[bi];
                let action = b.action.as_deref().and_then(parse_button_action);
                self.add_window_button(&b.text, b.x, b.y, b.width, b.height, &b.color, action);
                bi += 1;
            } else {
                let l = &due_lines[li];
                self.add_window_line((l.x1, l.y1), (l.x2, l.y2), &l.color);
                li += 1;
            }
        }
        let old_present = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .is_some();
        // A rebuilt display re-attaches the finsts of the features that were
        // attended when it was cleared (`transfer-finst`, `vision.lisp:2306`),
        // before the auto-stuff searches it. `process-display` then runs
        // `check-finsts` again, so a transfer older than the finst span does
        // not keep the rebuilt feature marked attended.
        self.apply_finst_transfers();
        self.check_visual_finsts();
        if self.can_stuff_visual() && self.tracked.is_none() {
            if let Some(feat) = self.newest_feature()? {
                let base = feat.name.clone();
                self.place_vl_at(&base, false, 10, old_present)?;
                // `stuff-visloc-buffer` schedules `unlock-vision` at priority
                // 9, just before its priority-10 set (`vision.lisp:1989`).
                // The hidden event counts toward `run-n-events`, which is why
                // `nearest-vis-loc`'s `run-n-events 2` stops before the SET.
                self.now_m(Prio::Num(12), Out::Never, true, Event::Dummy);
                self.now_m(Prio::Num(11), Out::Never, true, Event::Dummy);
            }
        }
        // Tracking re-syncs the vision buffers to the moved object
        // (`update-tracking-mth`) before the visicon update is announced.
        if self.tracked.is_some() {
            self.update_tracking(true);
        }
        self.now(Prio::Num(0), Out::Medium, Event::VisiconUpdate);
        // Schedule the next wave for remaining texts, moves or clears.
        let mut next_at: Option<i64> = None;
        if let Some(next) = self.pending_texts.first() {
            next_at = Some(next.at_ms);
        }
        if let Some(m) = self.pending_modifies.first() {
            next_at = Some(next_at.map_or(m.at_ms, |n| n.min(m.at_ms)));
        }
        if let Some(clear) = self.pending_clears.first() {
            next_at = Some(next_at.map_or(*clear, |n| n.min(*clear)));
        }
        if let Some(b) = self.pending_buttons.first() {
            next_at = Some(next_at.map_or(b.at_ms, |n| n.min(b.at_ms)));
        }
        if let Some(l) = self.pending_lines.first() {
            next_at = Some(next_at.map_or(l.at_ms, |n| n.min(l.at_ms)));
        }
        if let Some(m) = self.pending_line_modifies.first() {
            next_at = Some(next_at.map_or(m.at_ms, |n| n.min(m.at_ms)));
        }
        if let Some(r) = self.pending_line_removes.first() {
            next_at = Some(next_at.map_or(r.at_ms, |n| n.min(r.at_ms)));
        }
        if let Some(r) = self.pending_removes.first() {
            next_at = Some(next_at.map_or(r.at_ms, |n| n.min(r.at_ms)));
        }
        // A button re-add marks a window rebuild — the task's `build-display`
        // at the start of a new `run`. In ACT-R that is a *command the harness
        // runs between runs*, not a scheduled event, so the current run must
        // drain there and the new window be drawn when the next run starts
        // (`schedule_display`). Chaining a display to it would instead apply
        // the rebuild inside the previous run (`bst-learn`, whose 15 trials
        // share one `reset`).
        if let Some(b) = self.pending_buttons.first() {
            if next_at == Some(b.at_ms) {
                next_at = None;
            }
        }
        if let Some(at) = next_at {
            self.push_m(at, Prio::Num(-100), Out::Always, true, Event::ProcDisplay);
        }
        // A display wave only coalesces ops made *while it is running*. Once
        // the wave is done, `display_queued_ms` must describe the wave still
        // queued, if any: a client `clear-exp-window` followed immediately by
        // an add at this same time needs its own wave.
        self.display_queued_ms = (next_at == Some(now)).then_some(now);
        Ok(())
    }

    /// Place a visual-location chunk for a feature: the trace shows the
    /// requested base name, but the buffer holds a `-k` copy (which is
    /// what later requests bind). Aliased back to the feature.
    pub(super) fn place_vl(&mut self, base: &str, requested: bool) -> Result<(), EngineError> {
        self.place_vl_at(base, requested, 10, false)
    }

    pub(super) fn place_vl_at(
        &mut self,
        base: &str,
        requested: bool,
        prio: i64,
        overwrite: bool,
    ) -> Result<(), EngineError> {
        let idx = self
            .features
            .iter()
            .position(|f| f.name == base)
            .ok_or_else(|| bad(format!("no such visicon feature {base}")))?;
        let feat = self.features[idx].clone();
        // Copy counter starts at 0 and advances after use
        // (`new-name-fct` semantics; existing-name skips cannot occur
        // in fresh models).
        let n = *self.copy_count.entry(base.to_string()).or_insert(0);
        self.copy_count.insert(base.to_string(), n + 1);
        let copy = format!("{base}-{n}");
        self.vl_copies.insert(copy.clone(), idx);
        self.vl_copied_from.insert(copy.clone(), base.to_string());
        self.vl_xy.insert(copy.clone(), (feat.x, feat.y));
        self.vl_loc_spec.insert(copy.clone(), feat.loc_slots());
        let inst = ChunkInst {
            name: Some(copy.clone()),
            ctype: "VISUAL-LOCATION".to_string(),
            slots: feat.loc_slots(),
            bare: false,
        };
        self.buffer_mut("VISUAL-LOCATION").failed = false;
        self.buffer_mut("VISUAL-LOCATION").modified = false;
        if overwrite {
            self.now(
                Prio::Num(prio),
                Out::Low,
                Event::OverwriteBuffer {
                    buffer: "VISUAL-LOCATION".to_string(),
                    chunk: NewChunk::Instance(inst),
                    requested,
                    shown: Some(base.to_string()),
                    module: "VISION".to_string(),
                    copy_name: Some(copy.clone()),
                },
            );
        } else {
            self.now(
                Prio::Num(prio),
                Out::Low,
                Event::SetBuffer {
                    buffer: "VISUAL-LOCATION".to_string(),
                    chunk: NewChunk::Instance(inst),
                    requested,
                    shown: Some(base.to_string()),
                },
            );
        }
        // `stuff-visloc-buffer` (`vision.lisp:1995`): an auto-stuffed
        // (unrequested) location is unstuffed after `new-span`, unless
        // `:unstuff-visual-location` is nil.
        if !requested {
            let delay = match self.cfg.unstuff_vl {
                UnstuffVl::Off => None,
                UnstuffVl::NewSpan => Some(self.cfg.onset_span_ms),
                UnstuffVl::Delay(ms) => Some(ms),
            };
            if let Some(delay) = delay {
                self.push_m(
                    self.time_ms + delay,
                    Prio::Num(0),
                    // `unstuff-buffer` schedules `erase-buffer` with
                    // `schedule-event-now`'s default `:output t`
                    // (`support/general-pm.lisp:899`, `scheduling.lisp:1292`):
                    // high detail only.
                    Out::Always,
                    true,
                    Event::UnstuffVl { copy: copy.clone() },
                );
            }
        }
        Ok(())
    }

    /// The feature name a visual-location copy aliases (`chunk-visicon-entry`).
    pub(super) fn loc_feature(&self, copy: &str) -> Option<String> {
        // The base *name*, not `vl_copies`' feature index: a removal shifts
        // the feature vector and would leave the index pointing elsewhere.
        self.vl_copied_from.get(copy).cloned()
    }

    /// The source chunk a visual-buffer copy was made from
    /// (`chunk-copied-from-fct`), for the `vis-obj-marker` test: the
    /// `VISUAL-CHUNK0` copy of `VO-1` is not the tracked object.
    pub(super) fn obj_source(&self, name: &str) -> String {
        self.vl_copied_from
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// `convert-loc-to-object`: build a fresh visual-object chunk for the
    /// tracked feature. The object counter advances on every tracking
    /// update even when the chunk is only used for a modification
    /// (`vision.lisp:2762`).
    pub(super) fn convert_loc_to_object(&mut self, feat: &Feature) -> ChunkInst {
        let k = self.obj_built.entry(feat.kind.clone()).or_insert(0);
        let name = format!("{}{}", feat.kind, *k);
        *k += 1;
        self.obj_feature.insert(name.clone(), feat.name.clone());
        self.obj_feature
            .insert("VISUAL-CHUNK0".to_string(), feat.name.clone());
        let pos = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone())
            .unwrap_or_else(|| feat.name.clone());
        ChunkInst {
            name: Some(name),
            // The object's `isa` is `(visual-object <kind>)` (`vision.lisp`:
            // `(chunk-type line (visual-object))`), so the kind is the stored
            // type and `visual-object` matches it as a supertype.
            ctype: feat.kind.clone(),
            slots: Self::object_slots(feat, &pos),
            bare: false,
        }
    }

    /// The visual-object chunk slots (`SCREEN-POS`, `VALUE`, `COLOR`,
    /// `HEIGHT`, `WIDTH`, and the kind marker `TEXT`/`OVAL`/`LINE`).
    pub(super) fn object_slots(feat: &Feature, screen_pos: &str) -> Vec<(String, Value)> {
        let color = feat
            .spec
            .iter()
            .chain(feat.spec_pre.iter())
            .chain(feat.extra.iter())
            .find(|(k, _)| k == "COLOR")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| Value::Sym("BLACK".to_string()));
        let mut slots = vec![
            ("SCREEN-POS".to_string(), Value::Sym(screen_pos.to_string())),
            ("VALUE".to_string(), Value::Str(feat.value.clone())),
        ];
        if feat.kind != "IMAGE" {
            slots.push(("COLOR".to_string(), color));
        }
        slots.extend([
            ("HEIGHT".to_string(), Value::Int(feat.height)),
            ("WIDTH".to_string(), Value::Int(feat.width)),
            (feat.kind.clone(), Value::Sym("T".to_string())),
        ]);
        slots
    }

    /// `set-vis-loc`: stuff the tracked object's location chunk.
    pub(super) fn track_set_vis_loc(&mut self, feature: &str) {
        let _ = self.place_vl_at(feature, false, 15, false);
    }

    /// `mod-vis-loc`: refresh the location chunk in place. A modified chunk
    /// is no longer a copy (`chunk-copied-from-fct` clears).
    pub(super) fn track_mod_vis_loc(&mut self, feature: &str) {
        if let Some(cur) = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone())
        {
            self.vl_copied_from.remove(&cur);
        }
        if let Some(f) = self.features.iter().find(|f| f.name == feature).cloned() {
            self.now(
                Prio::Num(15),
                Out::Always,
                Event::ModBuffer {
                    buffer: "VISUAL-LOCATION".to_string(),
                    slots: vec![
                        ("SCREEN-X".to_string(), Value::Int(f.x)),
                        ("SCREEN-Y".to_string(), Value::Int(f.y)),
                    ],
                    module: "VISION".to_string(),
                },
            );
        }
    }

    /// The copy name `place_vl_at` will use for `base` next.
    pub(super) fn next_vl_copy(&self, base: &str) -> String {
        format!("{base}-{}", *self.copy_count.get(base).unwrap_or(&0))
    }

    /// `set-vis-obj`: stuff the fresh object into the visual buffer at the
    /// location chunk `pos`. The buffer holds a `VISUAL-CHUNK0` copy; the
    /// trace shows the source object name. `copy` marks the `(null
    /// vis-obj-chunk)` branch, which copies (`chunk-copied-from-fct`).
    pub(super) fn track_set_vis_obj(&mut self, obj: &ChunkInst, pos: &str, copy: bool) {
        let source = obj.name.clone().unwrap_or_default();
        let mut obj = obj.clone();
        obj.name = Some("VISUAL-CHUNK0".to_string());
        if let Some((_, v)) = obj.slots.iter_mut().find(|(k, _)| k == "SCREEN-POS") {
            *v = Value::Sym(pos.to_string());
        }
        if copy {
            self.vl_copied_from
                .insert("VISUAL-CHUNK0".to_string(), source.clone());
        } else {
            self.vl_copied_from.remove("VISUAL-CHUNK0");
        }
        self.now(
            Prio::Num(14),
            Out::Always,
            Event::SetBuffer {
                buffer: "VISUAL".to_string(),
                chunk: NewChunk::Instance(obj),
                requested: false,
                shown: Some(source),
            },
        );
    }

    /// `mod-vis-obj`: refresh the object's `screen-pos`. A modified chunk is
    /// no longer a copy (`chunk-copied-from-fct` clears).
    pub(super) fn track_mod_vis_obj(&mut self, pos: &str) {
        self.vl_copied_from.remove("VISUAL-CHUNK0");
        self.now(
            Prio::Num(14),
            Out::Always,
            Event::ModBuffer {
                buffer: "VISUAL".to_string(),
                slots: vec![("SCREEN-POS".to_string(), Value::Sym(pos.to_string()))],
                module: "VISION".to_string(),
            },
        );
    }

    /// `update-tracking-loc-chunk`: record the location chunk; `modify` also
    /// refreshes the visual object's `screen-pos` (silently, `:output nil`).
    pub(super) fn track_update_loc(&mut self, modify: bool, pos: &str) {
        self.tracked_lastloc = Some(pos.to_string());
        if modify {
            self.now(
                Prio::Num(13),
                Out::Never,
                Event::ModBuffer {
                    buffer: "VISUAL".to_string(),
                    slots: vec![("SCREEN-POS".to_string(), Value::Sym(pos.to_string()))],
                    module: "VISION".to_string(),
                },
            );
        }
    }

    /// `tracking-failed`: the tracked object left the visicon; clear the
    /// visual buffer and stop tracking.
    pub(super) fn tracking_failed(&mut self) {
        self.tracked = None;
        self.tracked_lastloc = None;
        self.last_attention = None;
        self.visual_busy = false;
        self.now(Prio::Num(13), Out::Always, Event::VisualClear);
        self.buffer_mut("VISUAL").failed = true;
    }

    /// `update-tracking-mth` (`vision.lisp:2706`): re-sync the vision
    /// buffers to the tracked object's new position.
    pub(super) fn update_tracking(&mut self, _from_proc_display: bool) {
        let Some(tracked) = self.tracked.clone() else {
            return;
        };
        let Some(feat) = self.features.iter().find(|f| f.name == tracked).cloned() else {
            self.tracking_failed();
            return;
        };
        let new_obj = self.convert_loc_to_object(&feat);
        let vis_loc = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone());
        let vis_obj = self
            .buffers
            .get("VISUAL")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone());
        let obj_marker = vis_obj
            .as_ref()
            .and_then(|n| self.obj_feature.get(&self.obj_source(n)).cloned());
        let mut old_loc = self.tracked_lastloc.clone();
        if old_loc.is_none() {
            if let Some(vlc) = &vis_loc {
                if self.loc_feature(vlc).as_deref() == Some(tracked.as_str()) {
                    old_loc = Some(vlc.clone());
                }
            }
        }
        let loc_is_old = vis_loc.is_some() && vis_loc == old_loc;
        let obj_is_tracked = obj_marker.as_deref() == Some(tracked.as_str());
        // The location chunk the buffers hold after a `set-vis-loc`.
        let stuffed = self.next_vl_copy(&tracked);
        if vis_loc.is_none() && vis_obj.is_none() {
            self.track_set_vis_loc(&tracked);
            self.track_set_vis_obj(&new_obj, &stuffed, false);
            self.track_update_loc(true, &stuffed);
        } else if vis_loc.is_none() && obj_is_tracked {
            self.track_set_vis_loc(&tracked);
            self.track_mod_vis_obj(&stuffed);
            self.track_update_loc(true, &stuffed);
        } else if vis_loc.is_none() {
            self.track_set_vis_loc(&tracked);
            self.track_update_loc(false, &stuffed);
        } else if loc_is_old && vis_obj.is_none() {
            let loc = vis_loc.clone().unwrap();
            self.track_mod_vis_loc(&tracked);
            self.track_set_vis_obj(&new_obj, &loc, false);
            self.track_update_loc(true, &loc);
        } else if loc_is_old && obj_is_tracked {
            let loc = vis_loc.clone().unwrap();
            self.track_mod_vis_loc(&tracked);
            self.track_mod_vis_obj(&loc);
            self.track_update_loc(true, &loc);
        } else if loc_is_old {
            self.track_mod_vis_loc(&tracked);
        } else if vis_obj.is_none() {
            // The visual buffer holds a non-tracked location, so the object's
            // `screen-pos` is the tracked feature's own chunk.
            self.track_set_vis_obj(&new_obj, &tracked, true);
        } else if obj_is_tracked {
            // The visual buffer holds the tracked object but the location
            // buffer holds some other chunk, so `screen-pos` is the tracked
            // feature's own chunk.
            self.track_mod_vis_obj(&tracked);
        }
    }

    /// `checknew` (`vision.lisp:2227`): a feature counts as `new` only until
    /// `:visual-onset-span` (default 0.5 s) has elapsed since its tstamp, and
    /// it is cleared early once it is attended (a finst is assigned).
    pub(super) fn feature_is_new(&self, f: &Feature) -> bool {
        f.new_p && self.time_ms - f.tstamp_ms <= self.cfg.onset_span_ms
    }

    pub(super) fn newest_feature(&mut self) -> Result<Option<Feature>, EngineError> {
        // `stuff-visloc-buffer` (`vision.lisp:1978`) calls
        // `find-current-locs-with-spec` with the vision module's
        // `default-spec` and then picks among the newest timestamps with
        // `random-item`. The module default is `screen-x lowest :attended new`
        // (`vision.lisp:3178`); `set-visloc-default` replaces it wholesale
        // (`vision.lisp:4057`), e.g. AGI's `kind oval screen-x lowest`.
        let spec = self.default_visloc_spec();
        let (plain, minmax): (Vec<_>, Vec<_>) = spec
            .into_iter()
            .partition(|(_, v)| !matches!(v, Value::Sym(s) if s == "LOWEST" || s == "HIGHEST"));
        let mut cands: Vec<Feature> = Vec::new();
        for f in &self.features {
            if loc_matches(f, &plain, self.time_ms, self.cfg.onset_span_ms)? {
                cands.push(f.clone());
            }
        }
        for (slot, v) in minmax {
            let lowest = matches!(&v, Value::Sym(s) if s == "LOWEST");
            let slot = slot.trim_start_matches(':');
            let best = cands.iter().filter_map(|f| feature_num(f, slot)).fold(
                None,
                |acc: Option<f64>, x| {
                    Some(match acc {
                        None => x,
                        Some(a) if (lowest && x < a) || (!lowest && x > a) => x,
                        Some(a) => a,
                    })
                },
            );
            if let Some(best) = best {
                cands.retain(|f| feature_num(f, slot) == Some(best));
            }
        }
        if cands.is_empty() {
            return Ok(None);
        }
        let top = cands.iter().map(|f| f.tstamp_ms).max().unwrap();
        let mut names: Vec<String> = cands
            .iter()
            .filter(|f| f.tstamp_ms == top)
            .map(|f| f.name.clone())
            .collect();
        names.sort();
        // `random-item` consumes exactly one draw, even for singletons.
        let pick = self.rng_draw(names.len());
        let want = names[pick].clone();
        Ok(self.features.iter().find(|f| f.name == want).cloned())
    }

    fn default_visloc_spec(&self) -> Vec<(String, Value)> {
        if self.model.visloc_default.is_empty() {
            return vec![
                ("SCREEN-X".to_string(), Value::Sym("LOWEST".to_string())),
                (":ATTENDED".to_string(), Value::Sym("NEW".to_string())),
            ];
        }
        self.model
            .visloc_default
            .iter()
            .filter_map(|(k, v)| Value::from_sexp(v).ok().map(|v| (k.clone(), v)))
            .collect()
    }

    /// `assign-finst` / `add-finst` (`vision.lisp:2282`): record or refresh the
    /// feature's finst and enforce the `:visual-num-finsts` cap. Exceeding the
    /// cap evicts the OLDEST finst, so a long-ago attended location becomes
    /// `:attended nil` again — this is what lets Sperling's fifth find
    /// reselect the auto-stuffed `VISUAL-LOCATION1`.
    pub(super) fn add_visual_finst(&mut self, idx: usize) {
        if self.features[idx].finst_ms.is_none() {
            let cap = self.cfg.visual_num_finsts;
            let live = self
                .features
                .iter()
                .filter(|f| f.finst_ms.is_some())
                .count();
            if live >= cap {
                if let Some(old) = self
                    .features
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.finst_ms.is_some())
                    .min_by_key(|(_, f)| f.finst_ms.unwrap())
                    .map(|(i, _)| i)
                {
                    self.features[old].finst_ms = None;
                }
            }
        }
        self.features[idx].finst_ms = Some(self.time_ms);
        self.features[idx].new_p = false;
    }

    /// `check-finsts` (`vision.lisp:2247`): drop finsts older than
    /// `:visual-finst-span`.
    pub(super) fn check_visual_finsts(&mut self) {
        let now = self.time_ms;
        let span = self.cfg.visual_finst_span_ms;
        for f in &mut self.features {
            if let Some(t) = f.finst_ms {
                if now - t > span {
                    f.finst_ms = None;
                }
            }
        }
    }

    /// Random index uniform in 0..n, always consuming exactly one draw
    /// (`random-item` draws even for singletons).
    pub(super) fn rng_draw(&mut self, n: usize) -> usize {
        self.rng.borrow_mut().random_int(n as u64) as usize
    }

    /// The `:nearest` post-filter from `find-current-locs-with-spec`
    /// (`vision.lisp:2137`). It narrows the candidates to the closest by the
    /// selected coordinate/distance, or to the first/last angular candidate
    /// around the module's center point.
    fn apply_nearest_filter(
        &mut self,
        cands: &mut Vec<Feature>,
        nearest: &Value,
        center: Option<&Value>,
    ) {
        let name = match nearest {
            Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
            _ => return,
        };
        if self.last_attention.is_none() {
            let msg = match name.as_str() {
                "CURRENT" | "CURRENT-X" | "CURRENT-Y" | "CURRENT-DISTANCE" => {
                    "No location has yet been attended so current is assumed to be at 0,0,1080."
                }
                "CLOCKWISE" | "COUNTERCLOCKWISE" => {
                    "No location has yet been attended so current is assumed to be at 0,0."
                }
                _ => "",
            };
            if !msg.is_empty() {
                self.emit_warning(msg);
            }
        }
        let current = self
            .last_attention
            .as_ref()
            .map(|loc| self.emma_loc_xyz(loc))
            .unwrap_or((0.0, 0.0, 1080.0));
        let center = match center {
            Some(Value::Sym(n)) | Some(Value::Str(n)) if self.loc_known(n) => {
                let p = self.emma_loc_xyz(n);
                (p.0, p.1)
            }
            _ => self.visual_center,
        };
        let point = if matches!(
            name.as_str(),
            "CURRENT"
                | "CURRENT-X"
                | "CURRENT-Y"
                | "CURRENT-DISTANCE"
                | "CLOCKWISE"
                | "COUNTERCLOCKWISE"
        ) {
            None
        } else if self.loc_known(&name) {
            Some(self.emma_loc_xyz(&name))
        } else {
            return;
        };
        let maximize = name == "CLOCKWISE";
        let score = |f: &Feature| -> Option<f32> {
            Some(match name.as_str() {
                "CURRENT" => vec_dist((f.x as f32, f.y as f32, 1080.0), current),
                "CURRENT-X" => (f.x as f32 - current.0).abs(),
                "CURRENT-Y" => (f.y as f32 - current.1).abs(),
                "CURRENT-DISTANCE" => (1080.0f32 - current.2).abs(),
                "CLOCKWISE" | "COUNTERCLOCKWISE" => {
                    angle_between_points(center, (current.0, current.1), (f.x as f32, f.y as f32))
                }
                _ => vec_dist((f.x as f32, f.y as f32, 1080.0), point.unwrap_or(current)),
            })
        };
        let best = cands.iter().filter_map(|f| score(f)).fold(None, |acc, x| {
            Some(match acc {
                None => x,
                Some(a) if (maximize && x > a) || (!maximize && x < a) => x,
                Some(a) => a,
            })
        });
        if let Some(best) = best {
            cands.retain(|f| score(f).is_some_and(|s| (s - best).abs() <= 1e-6));
        }
    }

    /// Find-location: newest-tstamp candidates, random among them,
    /// construct a fresh (dedup-named) location chunk into the buffer.
    pub(super) fn find_location(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        let (want, rels, neg) = match &spec {
            NewChunk::Spec {
                slots, rels, neg, ..
            } => (slots.clone(), rels.clone(), neg.clone()),
            _ => return Err(unimpl("non-spec visual-location request")),
        };
        // `find-current-locs-with-spec` (`vision.lisp:2088`) remaps every
        // `current` value to the current marker's slot before matching; a
        // slot missing from the marker drops the test entirely. `:nearest`
        // and `:center` are request parameters, not chunk slots, and are
        // applied after the ordinary match.
        let mut remapped_want: Vec<(String, Value)> = Vec::new();
        let mut nearest: Option<Value> = None;
        let mut center: Option<Value> = None;
        for (s, v) in want {
            match s.trim_start_matches(':').to_uppercase().as_str() {
                "NEAREST" => nearest = Some(v),
                "CENTER" => center = Some(v),
                _ if matches!(&v, Value::Sym(x) if x.eq_ignore_ascii_case("CURRENT")) => {
                    if let Some(value) = self.current_loc_slot(s.trim_start_matches(':')) {
                        remapped_want.push((s, value));
                    }
                }
                _ => remapped_want.push((s, v)),
            }
        }
        let want = remapped_want;
        let mut remapped_rels: Vec<(String, Value, SlotOp)> = Vec::new();
        for (s, v, op) in rels {
            if matches!(&v, Value::Sym(x) if x.eq_ignore_ascii_case("CURRENT")) {
                if let Some(value) = self.current_loc_slot(s.trim_start_matches(':')) {
                    remapped_rels.push((s, value, op));
                }
            } else {
                remapped_rels.push((s, v, op));
            }
        }
        let rels = remapped_rels;
        // `find-current-locs-with-spec` (`vision.lisp:2088`): a `slot lowest` /
        // `slot highest` value is pulled out of the match set and applied as a
        // post-filter (the candidate(s) with the min/max numeric value of that
        // slot).
        let mut plain = Vec::new();
        let mut minmax: Vec<(String, bool)> = Vec::new();
        for (s, v) in &want {
            let slot = s.trim_start_matches(':').to_uppercase();
            match v {
                Value::Sym(x) if x == "LOWEST" => minmax.push((slot, true)),
                Value::Sym(x) if x == "HIGHEST" => minmax.push((slot, false)),
                _ => plain.push((s.clone(), v.clone())),
            }
        }
        let mut cands: Vec<Feature> = Vec::new();
        for f in &self.features {
            let rejected =
                !neg.is_empty() && loc_matches(f, &neg, self.time_ms, self.cfg.onset_span_ms)?;
            if !rejected
                && loc_matches(f, &plain, self.time_ms, self.cfg.onset_span_ms)?
                && loc_rel_matches(f, &rels)
            {
                cands.push(f.clone());
            }
        }
        for (slot, lowest) in &minmax {
            let val = |f: &Feature| feature_num(f, slot);
            let best = cands
                .iter()
                .filter_map(val)
                .fold(None, |acc: Option<f64>, x| {
                    Some(match acc {
                        None => x,
                        Some(a) if (*lowest && x < a) || (!*lowest && x > a) => x,
                        Some(a) => a,
                    })
                });
            if let Some(b) = best {
                cands.retain(|f| val(f) == Some(b));
            }
        }
        if let Some(nearest) = nearest {
            self.apply_nearest_filter(&mut cands, &nearest, center.as_ref());
        }
        if cands.is_empty() {
            self.loc_failure = true;
            self.buffer_mut("VISUAL-LOCATION").failed = true;
            self.now(Prio::Num(0), Out::Low, Event::FindLocFailure);
            return Ok(());
        }
        let top = cands.iter().map(|f| f.tstamp_ms).max().unwrap();
        // `find-location` picks `(random-item (objs-max-val it
        // 'chunk-visual-tstamp))` (`vision.lisp:2923`) with NO sort: the
        // candidates are the newest-tstamp features in visicon order.
        let names: Vec<String> = cands
            .iter()
            .filter(|f| f.tstamp_ms == top)
            .map(|f| f.name.clone())
            .collect();
        // `random-item` consumes exactly one draw, even for singletons.
        let pick = self.rng_draw(names.len());
        let feat = cands
            .iter()
            .find(|f| f.name == names[pick])
            .cloned()
            .unwrap();
        let base = feat.name.clone();
        self.loc_failure = false;
        self.buffer_mut("VISUAL-LOCATION").failed = false;
        self.place_vl(&base, true)?;
        Ok(())
    }

    /// Attention landed: build the TEXT object and fill the buffer.
    pub(super) fn encoding_complete(
        &mut self,
        loc: String,
        requested: bool,
    ) -> Result<(), EngineError> {
        // `encoding-complete` (`vision.lisp:2413`) builds the object with
        // `get-obj-at-location` -> `within-move` (`vision.lisp:2485/2367`),
        // i.e. by the attention position, not the location chunk's name.
        // The requested encode's location is current, so its name resolves;
        // `update-attended-loc`'s re-encode (`requested = nil`) must match by
        // position: the old location chunk is gone (a window clear) and the
        // new object at the same visicon slot must not be mistaken for it
        // (paired-learning: `xray` -> `8` shift the centre past tolerance,
        // so nothing is found).
        let idx = if requested {
            match self.resolve_loc(&loc) {
                Ok(i) if i < self.features.len() => i,
                _ => return self.encoding_no_object(),
            }
        } else {
            let clof = self.emma_loc_xyz(&loc);
            match self.features.iter().position(|f| {
                angle_between(clof, (f.x as f32, f.y as f32, clof.2)) <= VISUAL_MOVEMENT_TOLERANCE
            }) {
                Some(i) => i,
                None => return self.encoding_no_object(),
            }
        };
        let feat = self.features[idx].clone();
        // `find-best-feature` (`vision.lisp:2573`) short-circuits when the
        // location chunk's visicon entry is still live — the same feature,
        // same timestamp, unmodified copy — and otherwise selects among the
        // nearest features with `random-item`, consuming exactly one draw
        // (even for a singleton). The engine keeps its own object choice but
        // must consume that draw so later `:er` selections line up.
        //
        // The entry is live when a *live* feature shares the copy's location
        // slots: `define-chunks-fct` merges a rebuilt feature's location chunk
        // into the existing one and reassigns its `chunk-visicon-entry`
        // (`vision.lisp:3883`), so `bst-learn`'s rebuilt display at the same
        // place keeps the entry live while `tutorial-unit3modeling-v6`'s
        // cleared window (a new object elsewhere) leaves it stale.
        let entry_live = self
            .vl_loc_spec
            .get(&loc)
            .map(|spec| self.features.iter().any(|f| &f.loc_slots() == spec))
            .unwrap_or(false);
        if !entry_live {
            self.rng_draw(1);
        }
        let k = self.obj_built.entry(feat.kind.clone()).or_insert(0);
        let source = format!("{}{}", feat.kind, *k);
        *k += 1;
        // The VISUAL buffer holds a `VISUAL-CHUNK{n}` copy; the trace shows
        // the source object name (`TEXT{n}`).
        self.obj_feature.insert(source.clone(), feat.name.clone());
        self.obj_feature
            .insert("VISUAL-CHUNK0".to_string(), feat.name.clone());
        let inst = ChunkInst {
            name: Some("VISUAL-CHUNK0".to_string()),
            // The object's `isa` is `(visual-object <kind>)` (`vision.lisp`:
            // `(chunk-type line (visual-object))`), so the kind is the stored
            // type and `visual-object` matches it as a supertype.
            ctype: feat.kind.clone(),
            // `set-chunk-slot-value-fct` stores a chunk slot value under its
            // *true* name (`get-true-slot-value-name`,
            // `framework/chunks.lisp:1106`): a location chunk whose
            // `-k` copy merged into an earlier identical copy (the
            // auto-stuffed location re-stuffed by `find-location`, which
            // merges into DM on the intervening clear) prints as that
            // earlier copy (`removed-and-restored-visual-item`).
            slots: Self::object_slots(&feat, &self.resolve_alias(&loc)),
            bare: false,
        };
        self.add_visual_finst(idx);
        self.visual_busy = false;
        // The encoded object records the visicon chunk it was built from, so
        // `buffer-chunk` prints `VISUAL-CHUNK0 [TEXT1]` (`chunk-copied-from-fct`).
        self.vl_copied_from
            .insert("VISUAL-CHUNK0".to_string(), source.clone());
        self.now(
            Prio::Num(10),
            Out::Low,
            Event::SetBuffer {
                buffer: "VISUAL".to_string(),
                chunk: NewChunk::Instance(inst),
                requested,
                shown: Some(source),
            },
        );
        Ok(())
    }

    /// The `No visual-object found` branch of `encoding-complete`
    /// (`vision.lisp:2436`): the attended object is gone, so vision reports
    /// no object, frees the module, and leaves VISUAL unchanged.
    pub(super) fn encoding_no_object(&mut self) -> Result<(), EngineError> {
        if self.cfg.detail.shows(Out::Medium) {
            self.emit("VISION", "No visual-object found".into());
        }
        self.visual_busy = false;
        // Becoming free again wakes the armed conflict-resolution
        // (`conditions-met`).
        self.changed();
        Ok(())
    }
}

impl Feature {
    pub(super) fn loc_slots(&self) -> Vec<(String, Value)> {
        let mut v = vec![
            ("KIND".to_string(), Value::Sym(self.kind.clone())),
            ("VALUE".to_string(), Value::Sym(self.kind.clone())),
            ("COLOR".to_string(), Value::Sym("BLACK".to_string())),
            ("HEIGHT".to_string(), Value::Int(self.height)),
            ("WIDTH".to_string(), Value::Int(self.width)),
            ("SCREEN-X".to_string(), Value::Int(self.x)),
            ("SCREEN-Y".to_string(), Value::Int(self.y)),
            ("DISTANCE".to_string(), Value::Int(1080)),
            // `simple-size` (`vision.lisp:3698`): pixels→angle product,
            // rounded to 0.01.
            ("SIZE".to_string(), Value::Num(self.size())),
        ];
        v.extend(self.extra.iter().cloned());
        v
    }

    /// `simple-size` (`vision.lisp:3698`): `0.01 * round(angle(w) *
    /// angle(h) / 0.01)`, with `angle(px) = 2*atan(px/2, 1080)` in degrees
    /// and single-float arithmetic (`round` returns the integer quotient).
    /// Returns the `compute-vis-loc-size` fallback `1.0` when `width` or
    /// `height` is absent (the engine stores an absent slot as 0, and
    /// `simple-size` requires `numberp`).
    pub(super) fn size(&self) -> f64 {
        if self.width == 0 || self.height == 0 {
            return 1.0;
        }
        let ang = |px: i64| (2.0 * ((px as f64) / 2.0 / 1080.0).atan()).to_degrees();
        let prod = ang(self.width) * ang(self.height);
        let q = (prod / 0.01).round();
        (0.01f32 * q as f32) as f64
    }

    /// The feature's filled slots in `printed-visicon` order: the computed
    /// `size`/`distance` (only when the construction spec did not set them)
    /// followed by the spec order. `chunk-filled-slots-list-fct` is a chunk's
    /// slots in reverse set order and `define-chunks-fct` sets them in reverse
    /// spec order, so the printed list is `[SIZE, DISTANCE] ++ spec`
    /// (PORTING-NOTES #238).
    pub(super) fn filled_slots(&self) -> Vec<(String, Value)> {
        let has = |n: &str| {
            self.spec.iter().any(|(k, _)| k == n) || self.spec_pre.iter().any(|(k, _)| k == n)
        };
        let mut v: Vec<(String, Value)> = self.spec_pre.clone();
        if !has("SIZE") {
            v.push(("SIZE".to_string(), Value::Num(self.size())));
        }
        if !has("DISTANCE") {
            v.push(("DISTANCE".to_string(), Value::Int(1080)));
        }
        v.extend(self.spec.iter().cloned());
        v
    }

    /// `set-chunk-slot-value-fct` on a visicon feature: an existing slot keeps
    /// its position, a new one is pushed onto the front of the printed list
    /// (`chunks.lisp:1459`).
    pub(super) fn set_slot(&mut self, name: &str, value: Value) {
        match (name, &value) {
            ("SCREEN-X", Value::Int(i)) => self.x = *i,
            ("SCREEN-Y", Value::Int(i)) => self.y = *i,
            ("WIDTH", Value::Int(i)) => self.width = *i,
            ("HEIGHT", Value::Int(i)) => self.height = *i,
            ("VALUE", Value::Str(s)) => self.value = s.clone(),
            ("KIND", Value::Sym(s)) => self.kind = s.clone(),
            _ => {}
        }
        if let Some(e) = self.spec.iter_mut().find(|(k, _)| k == name) {
            e.1 = value;
        } else if let Some(e) = self.spec_pre.iter_mut().find(|(k, _)| k == name) {
            e.1 = value;
        } else {
            self.spec_pre.insert(0, (name.to_string(), value));
        }
    }
}

/// The numeric value of a feature's slot for a `lowest`/`highest` request
/// constraint (`fast-chunk-slot-value-fct`, `vision.lisp:2124`); `None` when
/// the slot is not numeric (such candidates are ignored by the constraint).
fn feature_num(f: &Feature, slot: &str) -> Option<f64> {
    match slot {
        "SCREEN-X" => Some(f.x as f64),
        "SCREEN-Y" => Some(f.y as f64),
        "WIDTH" => Some(f.width as f64),
        "HEIGHT" => Some(f.height as f64),
        "SIZE" => Some(f.size()),
        _ => None,
    }
}

/// A visual feature's value for a slot request. The base fields are authoritative
/// for geometry/object slots; everything else comes from the feature's
/// construction spec.
fn feature_slot(f: &Feature, slot: &str) -> Option<Value> {
    match slot {
        "SCREEN-X" => Some(Value::Int(f.x)),
        "SCREEN-Y" => Some(Value::Int(f.y)),
        "WIDTH" => Some(Value::Int(f.width)),
        "HEIGHT" => Some(Value::Int(f.height)),
        "SIZE" => Some(Value::Num(f.size())),
        "VALUE" => Some(Value::Str(f.value.clone())),
        "KIND" => Some(Value::Sym(f.kind.clone())),
        _ => f
            .filled_slots()
            .into_iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(slot))
            .map(|(_, value)| value),
    }
}

/// ACT-R compares visual feature strings and symbols case-insensitively
/// (`bst` asks for `value "b"` while the visicon holds `"B"`).
fn feature_value_matches(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::Str(a), Value::Str(b))
        | (Value::Str(a), Value::Sym(b))
        | (Value::Sym(a), Value::Str(b))
        | (Value::Sym(a), Value::Sym(b)) => a.eq_ignore_ascii_case(b),
        _ => got == want,
    }
}

/// Visual-location request matching over features. Unconstrained
/// requests match everything; unknown slots error.
pub(super) fn loc_matches(
    feat: &Feature,
    want: &[(String, Value)],
    now_ms: i64,
    onset_span_ms: i64,
) -> Result<bool, EngineError> {
    for (s, v) in want {
        // Request parameters are written with a leading colon (`:attended`,
        // `:nearest`, ...); treat them as the plain slot for matching.
        let s = s.trim_start_matches(':');
        match (s, v) {
            ("ISA", _) => {}
            ("ATTENDED", Value::Sym(x)) if x == "T" => {
                if feat.finst_ms.is_none() {
                    return Ok(false);
                }
            }
            ("ATTENDED", Value::Sym(x)) if x == "NEW" => {
                if !(feat.new_p && now_ms - feat.tstamp_ms <= onset_span_ms) {
                    return Ok(false);
                }
            }
            ("ATTENDED", Value::Sym(x)) if x == "NIL" => {
                if feat.finst_ms.is_some() {
                    return Ok(false);
                }
            }
            ("ATTENDED", Value::Nil) => {
                if feat.finst_ms.is_some() {
                    return Ok(false);
                }
            }
            ("ATTENDED", _) => {
                return Err(unimpl("non-t/nil :attended"));
            }
            ("KIND", Value::Sym(k)) => {
                if *k != feat.kind {
                    return Ok(false);
                }
            }
            // ACT-R compares the text of a visual feature case-insensitively:
            // `bst` asks for `value "b"` / `value "reset"` and the visicon holds
            // `"B"` / `"Reset"` (`uwi.lisp:401`), and the ref shows the request
            // matching.
            ("VALUE", Value::Str(val)) => {
                if !val.eq_ignore_ascii_case(&feat.value) {
                    return Ok(false);
                }
            }
            ("VALUE", Value::Sym(val)) => {
                if !val.eq_ignore_ascii_case(&feat.value) {
                    return Ok(false);
                }
            }
            ("SCREEN-X", Value::Num(x)) => {
                if (*x as i64) != feat.x {
                    return Ok(false);
                }
            }
            ("SCREEN-Y", Value::Num(y)) => {
                if (*y as i64) != feat.y {
                    return Ok(false);
                }
            }
            // Device-feature slots (`color`, `width`, `text`, …) are ordinary
            // visual-location chunk slots; match them from the feature's
            // construction spec. A slot the feature does not carry cannot
            // match (`fast-chunk-slot-value-fct` returns nil).
            (
                "COLOR" | "WIDTH" | "HEIGHT" | "SIZE" | "DISTANCE" | "TEXT" | "LINE" | "OVAL"
                | "IMAGE" | "END1-X" | "END1-Y" | "END2-X" | "END2-Y",
                _,
            ) => {
                let Some(got) = feature_slot(feat, s) else {
                    return Ok(false);
                };
                if !feature_value_matches(&got, v) {
                    return Ok(false);
                }
            }
            got => {
                return Err(unimpl(format!("visual-location slot {got:?}")));
            }
        }
    }
    Ok(true)
}

/// Request slot relations (`> screen-y 450 < screen-y 470`) on a
/// visual-location search (`find-location`, `vision.lisp:2921`). Unknown
/// slots are not filtered (the search ignores them).
pub(super) fn loc_rel_matches(feat: &Feature, rels: &[(String, Value, SlotOp)]) -> bool {
    for (s, v, op) in rels {
        let s = s.trim_start_matches(':');
        let (got, want) = match (s, v) {
            ("SCREEN-X", Value::Num(x)) => (feat.x as f64, *x as f64),
            ("SCREEN-X", Value::Int(x)) => (feat.x as f64, *x as f64),
            ("SCREEN-Y", Value::Num(y)) => (feat.y as f64, *y as f64),
            ("SCREEN-Y", Value::Int(y)) => (feat.y as f64, *y as f64),
            _ => continue,
        };
        let ok = match op {
            SlotOp::Gt => got > want,
            SlotOp::Lt => got < want,
            SlotOp::Ge => got >= want,
            SlotOp::Le => got <= want,
            SlotOp::Eq | SlotOp::Neg => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

/// CL `round`: fractional .5 goes to the even neighbor.
/// Euclidean distance between two 3-vectors (`dist`, `misc-utils.lisp:1156`).
pub(super) fn vec_dist(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
}

/// `angle-between` (`vision.lisp:2045`) around a center point, in radians and
/// wrapped to `[0, 2π)`. ACT-R's `py` uses the GUI's upward-positive y, while
/// the visicon's `screen-y` grows downward, hence the negated y term.
/// Clockwise is the maximum angle (`vision.lisp:2041`).
fn angle_between_points(center: (f32, f32), p1: (f32, f32), p2: (f32, f32)) -> f32 {
    let angle = |p: (f32, f32)| (-(p.1 - center.1)).atan2(p.0 - center.0);
    let mut a = angle(p1) - angle(p2);
    if a < 0.0 {
        a += std::f32::consts::TAU;
    }
    a
}

/// `angle-between-vectors` (`vision.lisp:2386`) in degrees.
pub(super) fn angle_between(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let num = b.2;
    let den = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2) + b.2.powi(2)).sqrt();
    (num / den).clamp(-1.0, 1.0).acos().to_degrees()
}

/// `add-gaussian-noise` (`emma.lisp:730`): `x` plus a logistic sample of
/// scale `sqrt(3·stddev²)/π`.
pub(super) fn add_gaussian_noise(rng: &mut Mt, x: f32, stddev: f32) -> f32 {
    let s = (3.0f32 * stddev * stddev).sqrt() / std::f32::consts::PI;
    x + if s == 0.0 { 0.0 } else { rng.noise(s) }
}

fn audio_time_value(ms: i64, audio_ms_times: bool) -> Value {
    if audio_ms_times {
        Value::Int(ms)
    } else {
        Value::Num(ms as f64 / 1000.0)
    }
}

/// The sound's `audio-event` chunk (`update-sound-evt`, `audio.lisp:922`):
/// `onset`, optional `offset`/`duration`, `pitch`, `kind`, `location`, `id`,
/// then the `:evt`/`:both` custom slots. The model's `detected-sound` matches
/// it untested and copies it as `event`.
pub(super) fn audio_event_chunk(s: &Sound, now_ms: i64, audio_ms_times: bool) -> ChunkInst {
    let mut slots = vec![(
        "ONSET".to_string(),
        audio_time_value(s.onset, audio_ms_times),
    )];
    if let (Some(offset), Some(duration)) = (s.offset, s.duration) {
        if now_ms >= offset {
            slots.push((
                "OFFSET".to_string(),
                audio_time_value(offset, audio_ms_times),
            ));
            slots.push((
                "DURATION".to_string(),
                audio_time_value(duration, audio_ms_times),
            ));
        }
    }
    slots.extend([
        ("PITCH".to_string(), Value::Sym(s.pitch.clone())),
        ("KIND".to_string(), Value::Sym(s.kind.clone())),
        ("LOCATION".to_string(), Value::Sym(s.location.clone())),
        ("ID".to_string(), Value::Sym(s.ename.clone())),
    ]);
    for f in &s.other_features {
        if matches!(f.scope, AudioFeatureScope::Evt | AudioFeatureScope::Both) {
            slots.push((f.slot.clone(), f.value.clone()));
        }
    }
    ChunkInst {
        name: Some(s.ename.clone()),
        ctype: "AUDIO-EVENT".to_string(),
        slots,
        bare: false,
    }
}

/// The sound chunk (`audio-encoding-complete`, `audio.lisp:1205`):
/// `kind`, `content`, `event` (the `audio-event` chunk name), then the
/// `:sound`/`:both` custom slots.
pub(super) fn sound_chunk(s: &Sound) -> ChunkInst {
    let mut slots = vec![
        ("KIND".to_string(), Value::Sym(s.kind.clone())),
        ("CONTENT".to_string(), s.content.clone()),
        ("EVENT".to_string(), Value::Sym(s.ename.clone())),
    ];
    for f in &s.other_features {
        if matches!(f.scope, AudioFeatureScope::Sound | AudioFeatureScope::Both) {
            slots.push((f.slot.clone(), f.value.clone()));
        }
    }
    ChunkInst {
        name: Some(s.sname.clone()),
        ctype: "SOUND".to_string(),
        slots,
        bare: false,
    }
}

/// Parse an `add-button-to-exp-window` `:action` captured by the device
/// sidecar: `"command"` or `("command" arg ...)`.
fn parse_button_action(raw: &str) -> Option<(String, Vec<Json>)> {
    use crate::sexpr::{parse_all, Sexp};
    fn json(s: &Sexp) -> Json {
        match s {
            Sexp::Sym(x) | Sexp::Str(x) => Json::Str(x.clone()),
            Sexp::Int(i) => Json::Int(*i),
            Sexp::Float(x) => Json::Float(*x),
            Sexp::List(items) => Json::Arr(items.iter().map(json).collect()),
        }
    }
    match parse_all(raw).ok()?.first()? {
        Sexp::Sym(name) | Sexp::Str(name) => Some((name.clone(), Vec::new())),
        Sexp::List(items) => {
            let name = match items.first()? {
                Sexp::Sym(name) | Sexp::Str(name) => name.clone(),
                _ => return None,
            };
            Some((name, items[1..].iter().map(json).collect()))
        }
        _ => None,
    }
}
