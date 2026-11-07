//! Experiment-window rendering, input forwarding, and close behavior.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;

use crate::json::Json;

use super::super::*;
use super::support::*;

fn open_demo(app: &mut App) {
    app.apply_window_op(vec![
        Json::Str("open".into()),
        Json::Str("Demo".into()),
        Json::Int(10),
        Json::Int(20),
        Json::Int(300),
        Json::Int(200),
    ]);
}

#[test]
fn experiment_window_click_forwards_local_coordinates() {
    let (mut h, calls, _state) = mock_harness();
    h.run();
    open_demo(h.state_mut());
    h.run();

    let center = h
        .get_by_role_and_label(Role::Window, "Demo")
        .rect()
        .center();
    h.event(Event::PointerMoved(center));
    h.event(Event::PointerButton {
        pos: center,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::default(),
    });
    h.event(Event::PointerButton {
        pos: center,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::default(),
    });
    h.run();

    let call = calls
        .borrow()
        .iter()
        .find(|c| c.name == "visible-virtual-window-mouse-click")
        .cloned()
        .expect("window click was not forwarded");
    assert_eq!(call.args.first().and_then(Json::as_str), Some("Demo"));
    assert!(call.args.get(1).and_then(Json::as_f64).unwrap_or(-1.0) >= 0.0);
    assert!(call.args.get(2).and_then(Json::as_f64).unwrap_or(-1.0) >= 0.0);
}

#[test]
fn experiment_window_keyboard_forwards_space_and_special_keys() {
    let (mut h, calls, _state) = mock_harness();
    h.run();
    open_demo(h.state_mut());
    h.run();

    let center = h
        .get_by_role_and_label(Role::Window, "Demo")
        .rect()
        .center();
    h.event(Event::PointerMoved(center));
    h.event(Event::PointerButton {
        pos: center,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::default(),
    });
    h.event(Event::PointerButton {
        pos: center,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::default(),
    });
    h.run();

    h.event(Event::Text(" ".into()));
    h.run();
    h.key_press(Key::Enter);
    h.run();

    let calls = calls.borrow();
    let keys: Vec<&str> = calls
        .iter()
        .filter(|c| c.name == "output-key")
        .filter_map(|c| c.args.first())
        .filter_map(|a| match a {
            Json::Arr(items) => items.get(1).and_then(Json::as_str),
            _ => None,
        })
        .collect();
    assert!(keys.contains(&"space"), "keys: {keys:?}");
    assert!(keys.contains(&"return"), "keys: {keys:?}");
}

#[test]
fn experiment_window_close_and_close_all_issue_rpc() {
    let (mut h, calls, _state) = mock_harness();
    h.run();
    open_demo(h.state_mut());
    h.state_mut().apply_window_op(vec![
        Json::Str("open".into()),
        Json::Str("Other".into()),
        Json::Int(40),
        Json::Int(40),
        Json::Int(200),
        Json::Int(150),
    ]);
    h.run();
    assert_eq!(h.state().windows.len(), 2);

    h.get_by_role_and_label(Role::Window, "Demo")
        .get_by_label("Close window")
        .click_accesskit();
    h.run();
    assert_eq!(h.state().windows.len(), 1);
    assert!(calls.borrow().iter().any(|c| {
        c.name == "close-exp-window" && c.args.first().and_then(Json::as_str) == Some("Demo")
    }));

    h.get_by_role_and_label(Role::Button, "Close Exp Windows")
        .click_accesskit();
    h.run();
    assert!(h.state().windows.is_empty());
    assert!(calls
        .borrow()
        .iter()
        .any(|c| c.name == "close-all-exp-windows"));
}

#[test]
fn window_item_ops_render_and_remove() {
    let (mut h, _calls, _state) = mock_harness();
    h.run();
    {
        let app = h.state_mut();
        open_demo(app);
        app.apply_window_op(vec![
            Json::Str("text".into()),
            Json::Str("Demo".into()),
            Json::Str("label".into()),
            Json::Int(10),
            Json::Int(10),
            Json::Str("Hello".into()),
            Json::Str("black".into()),
            Json::Int(14),
        ]);
        app.apply_window_op(vec![
            Json::Str("button".into()),
            Json::Str("Demo".into()),
            Json::Str("button".into()),
            Json::Int(10),
            Json::Int(40),
            Json::Int(80),
            Json::Int(24),
            Json::Str("OK".into()),
            Json::Str("light-gray".into()),
        ]);
        app.apply_window_op(vec![
            Json::Str("attention".into()),
            Json::Str("Demo".into()),
            Json::Str("M".into()),
            Json::Int(30),
            Json::Int(40),
            Json::Str("red".into()),
        ]);
        app.apply_window_op(vec![
            Json::Str("cursor".into()),
            Json::Str("Demo".into()),
            Json::Str("M".into()),
            Json::Int(30),
            Json::Int(40),
            Json::Str("blue".into()),
        ]);
    }
    h.run();
    let window = &h.state().windows[0];
    assert_eq!(window.items.len(), 2);
    assert!(window.attention.is_some());
    assert!(window.cursor.is_some());

    {
        let app = h.state_mut();
        app.apply_window_op(vec![
            Json::Str("remove".into()),
            Json::Str("Demo".into()),
            Json::Str("label".into()),
        ]);
        app.apply_window_op(vec![
            Json::Str("clearattention".into()),
            Json::Str("Demo".into()),
            Json::Str("M".into()),
        ]);
        app.apply_window_op(vec![
            Json::Str("clearcursor".into()),
            Json::Str("Demo".into()),
            Json::Str("M".into()),
        ]);
    }
    h.run();
    let window = &h.state().windows[0];
    assert_eq!(window.items.len(), 1);
    assert!(window.attention.is_none());
    assert!(window.cursor.is_none());
}

#[test]
fn reopening_a_window_replaces_it_by_title() {
    let mut h = mock_harness().0;
    h.run();
    open_demo(h.state_mut());
    h.run();
    h.state_mut().apply_window_op(vec![
        Json::Str("open".into()),
        Json::Str("Demo".into()),
        Json::Int(50),
        Json::Int(60),
        Json::Int(100),
        Json::Int(80),
    ]);
    h.run();
    assert_eq!(h.state().windows.len(), 1);
    assert_eq!(h.state().windows[0].w, 100.0);
}
