//! Browser-facing session bridge over the RPC/AGI command layer.

use crate::engine::{HookValue, PpmHookFn, TraceKind};
use crate::json::Json;
use crate::rpc::session::{Remote, Session};

const WINDOW_HANDLER: &str = "web-vv";

#[derive(Default)]
struct WebRemote {
    events: Vec<Json>,
}

impl WebRemote {
    fn take(&mut self) -> Vec<Json> {
        std::mem::take(&mut self.events)
    }
}

impl Remote for WebRemote {
    fn call(
        &mut self,
        _name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        if matches!(args.first(), Some(Json::Str(op)) if op == "check") {
            return Ok(vec![Json::Bool(true)]);
        }
        self.events.push(Json::Obj(vec![
            ("type".into(), Json::Str("window".into())),
            (
                "model".into(),
                model.map(|m| Json::Str(m.into())).unwrap_or(Json::Null),
            ),
            ("args".into(), Json::Arr(args)),
        ]));
        Ok(vec![Json::Bool(true)])
    }
}

pub struct WebSession {
    session: Session,
    remote: WebRemote,
    events: Vec<Json>,
}

impl WebSession {
    pub fn new(seed: (u64, u64)) -> Result<WebSession, String> {
        let mut session = Session::new(seed);
        let mut remote = WebRemote::default();
        session.add_remote(WINDOW_HANDLER, Some(WINDOW_HANDLER.into()), None);
        session.evaluate(
            "add-virtual-window-handler",
            None,
            &[Json::Str(WINDOW_HANDLER.into())],
            &mut remote,
        )?;
        Ok(WebSession {
            session,
            remote,
            events: Vec::new(),
        })
    }

    pub fn load_source(&mut self, source: &str) -> Result<(), String> {
        self.session.load_source(source)?;
        self.collect();
        Ok(())
    }

    pub fn eval(&mut self, name: &str, args: &[Json]) -> Result<Vec<Json>, String> {
        let result = self.session.evaluate(name, None, args, &mut self.remote);
        self.collect();
        result
    }

    pub fn run_begin(&mut self, name: &str, secs: f64) -> (i64, bool, bool) {
        let result = self.session.run_begin(name, secs);
        self.collect();
        result
    }

    pub fn run_chunk(
        &mut self,
        limit_ms: i64,
        full: bool,
        max_events: u64,
    ) -> Result<bool, String> {
        let result = self
            .session
            .run_chunk(limit_ms, full, Some(max_events), None);
        self.collect();
        result
    }

    pub fn take_events(&mut self) -> Vec<Json> {
        std::mem::take(&mut self.events)
    }

    /// Install (or clear) the session-wide embedder `:sim-hook`.
    pub fn set_sim_hook(
        &mut self,
        hook: Option<std::rc::Rc<dyn Fn(&HookValue, &HookValue) -> Option<f32>>>,
    ) {
        self.session.set_sim_hook(hook);
    }

    /// Install (or clear) the session-wide embedder `:ppm-hook`.
    pub fn set_ppm_hook(&mut self, hook: Option<PpmHookFn>) {
        self.session.set_ppm_hook(hook);
    }

    /// A pending host `imaginal-action`, as `(command, args, simple)`.
    pub fn take_pending_host_action(&mut self) -> Option<(String, Vec<String>, bool)> {
        self.session.take_pending_host_action()
    }

    /// Complete a pending host `imaginal-action`.
    pub fn complete_host_action(
        &mut self,
        result: Option<&str>,
        simple: bool,
    ) -> Result<(), String> {
        self.session.complete_host_action(result, simple)
    }

    fn collect(&mut self) {
        self.events.extend(self.remote.take());

        for (name, model, key) in self.session.take_fired_commands() {
            self.events.push(Json::Obj(vec![
                ("type".into(), Json::Str("command".into())),
                ("name".into(), Json::Str(name)),
                ("model".into(), Json::Str(model)),
                ("key".into(), Json::Str(key)),
            ]));
        }

        for (name, args) in self.session.take_fired_task_commands() {
            self.events.push(Json::Obj(vec![
                ("type".into(), Json::Str("task-command".into())),
                ("name".into(), Json::Str(name)),
                (
                    "args".into(),
                    Json::Arr(args.into_iter().map(Json::Str).collect()),
                ),
            ]));
        }

        for (name, args) in self.session.take_pending_device_actions() {
            self.events.push(Json::Obj(vec![
                ("type".into(), Json::Str("device-action".into())),
                ("name".into(), Json::Str(name)),
                ("args".into(), Json::Arr(args)),
            ]));
        }

        for (line, kind) in self.session.take_trace_events() {
            self.events.push(Json::Obj(vec![
                ("type".into(), Json::Str("trace".into())),
                ("line".into(), Json::Str(line)),
                ("kind".into(), Json::Str(kind_name(kind).into())),
            ]));
        }
    }
}

fn kind_name(kind: TraceKind) -> &'static str {
    match kind {
        TraceKind::Model => "model",
        TraceKind::Command => "command",
        TraceKind::Warning => "warning",
        TraceKind::General => "general",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> WebSession {
        let mut session = WebSession::new((100, 0)).unwrap();
        session
            .load_source("(clear-all)\n(define-model test (sgp :v t))")
            .unwrap();
        session
    }

    #[test]
    fn visible_window_messages_are_queued() {
        let mut session = session();
        session
            .eval(
                "open-exp-window",
                &[
                    Json::Str("Test".into()),
                    Json::Obj(vec![
                        ("x".into(), Json::Int(10)),
                        ("y".into(), Json::Int(20)),
                        ("width".into(), Json::Int(100)),
                        ("height".into(), Json::Int(80)),
                    ]),
                ],
            )
            .unwrap();
        let events = session.take_events();
        assert!(events.iter().any(|event| {
            matches!(
                event,
                Json::Obj(items)
                    if items.iter().any(|(k, v)| k == "type" && v == &Json::Str("window".into()))
            )
        }));
    }

    #[test]
    fn event_queue_is_drained() {
        let mut session = session();
        session
            .eval("open-exp-window", &[Json::Str("Test".into())])
            .unwrap();
        assert!(!session.take_events().is_empty());
        assert!(session.take_events().is_empty());
    }

    #[test]
    fn stepped_run_dispatches_task_commands() {
        let mut session = session();
        session
            .eval("add-act-r-command", &[Json::Str("test-cmd".into())])
            .unwrap();
        session
            .eval(
                "schedule-event-relative",
                &[
                    Json::Float(0.05),
                    Json::Str("test-cmd".into()),
                    Json::Obj(vec![(
                        "params".into(),
                        Json::Arr(vec![Json::Obj(vec![(
                            "str".into(),
                            Json::Str("hello".into()),
                        )])]),
                    )]),
                ],
            )
            .unwrap();

        let (limit, full, should_run) = session.run_begin("run", 0.2);
        assert!(should_run);
        for _ in 0..16 {
            let more = session.run_chunk(limit, full, 1).unwrap();
            if !more {
                break;
            }
        }

        let events = Json::Arr(session.take_events()).write();
        assert!(events.contains("task-command"), "{events}");
        assert!(events.contains("test-cmd"), "{events}");
        assert!(events.contains("hello"), "{events}");
    }

    #[test]
    fn unit2_demo2_drives_visual_task() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../actr7.x/tutorial/unit2/demo2-model.lisp");
        let source = std::fs::read_to_string(path).unwrap();
        let mut session = WebSession::new((123456, 0)).unwrap();
        session.load_source(&source).unwrap();
        session.eval("reset", &[]).unwrap();

        let alphabet = Json::Arr(
            "BCDFGHJKLMNPQRSTVWXYZ"
                .chars()
                .map(|c| Json::Str(c.to_string()))
                .collect(),
        );
        let permuted = session.eval("permute-list", &[alphabet]).unwrap();
        let Json::Arr(items) = &permuted[0] else {
            panic!("permute-list did not return an array");
        };
        let text = items.first().and_then(Json::as_str).unwrap();
        assert_eq!(text, "V");

        session
            .eval("open-exp-window", &[Json::Str("Letter recognition".into())])
            .unwrap();
        session
            .eval(
                "add-text-to-exp-window",
                &[
                    Json::Str("Letter recognition".into()),
                    Json::Str(text.into()),
                    Json::Arr(vec![
                        Json::Arr(vec![Json::Str("x".into()), Json::Int(125)]),
                        Json::Arr(vec![Json::Str("y".into()), Json::Int(150)]),
                    ]),
                ],
            )
            .unwrap();
        session
            .eval("install-device", &[Json::Str("Letter recognition".into())])
            .unwrap();
        session.eval("run", &[Json::Int(10)]).unwrap();

        let events = Json::Arr(session.take_events()).write();
        assert!(events.contains("output-key DEMO2 v"), "{events}");
        assert!(events.contains("Letter recognition"), "{events}");
        assert!(events.contains("\"text\""), "{events}");
        assert!(events.contains("125,150"), "{events}");
    }
}
