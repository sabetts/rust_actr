//! Editor-window interactions that do not require a native file dialog.

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;

use super::super::*;
use super::support::*;

fn editor_title(path: &std::path::Path) -> String {
    path.display().to_string()
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-backup-0", path.display()));
}

#[test]
fn editor_typing_marks_dirty_and_save_writes() {
    let path = temp_path("editor-save.lisp");
    std::fs::write(&path, "(define-model old)").unwrap();
    let mut h = inproc_harness();
    h.run();
    h.state_mut().open_editor_path(path.clone());
    h.run();

    let title = editor_title(&path);
    h.get_by_role_and_label(Role::Window, &title)
        .get_by_role(Role::MultilineTextInput)
        .focus();
    h.run();
    h.get_by_role_and_label(Role::Window, &title)
        .get_by_role(Role::MultilineTextInput)
        .type_text("(define-model new)");
    h.run();

    let app = h.state();
    assert!(app.editors[0].dirty);
    let dirty_title = format!("{title} *");
    h.get_by_role_and_label(Role::Window, &dirty_title)
        .get_by_label("Save")
        .click_accesskit();
    h.run();

    assert!(!h.state().editors[0].dirty);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "(define-model old)(define-model new)"
    );
    cleanup(&path);
}

#[test]
fn save_all_writes_every_dirty_editor() {
    let first = temp_path("editor-all-1.lisp");
    let second = temp_path("editor-all-2.lisp");
    std::fs::write(&first, "one").unwrap();
    std::fs::write(&second, "two").unwrap();
    let mut h = inproc_harness();
    h.run();
    {
        let app = h.state_mut();
        app.open_editor_path(first.clone());
        app.open_editor_path(second.clone());
        app.editors[0].text = "one changed".into();
        app.editors[0].dirty = true;
        app.editors[1].text = "two changed".into();
        app.editors[1].dirty = true;
    }
    h.run();

    h.get_by_role_and_label(Role::Button, "Save All")
        .click_accesskit();
    h.run();

    assert_eq!(std::fs::read_to_string(&first).unwrap(), "one changed");
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "two changed");
    assert!(h.state().editors.iter().all(|e| !e.dirty));
    cleanup(&first);
    cleanup(&second);
}

#[test]
fn unsaved_editor_close_cancel_then_discard() {
    let path = temp_path("editor-close.lisp");
    std::fs::write(&path, "original").unwrap();
    let mut h = inproc_harness();
    h.run();
    h.state_mut().open_editor_path(path.clone());
    {
        let app = h.state_mut();
        app.editors[0].text = "changed".into();
        app.editors[0].dirty = true;
    }
    h.run();

    let dirty_title = format!("{} *", editor_title(&path));
    h.get_by_role_and_label(Role::Window, &dirty_title)
        .get_by_label("Close window")
        .click_accesskit();
    h.run();
    assert!(h.state().editor_close.is_some());
    h.get_by_role_and_label(Role::Window, "Unsaved changes")
        .get_by_label("Cancel")
        .click_accesskit();
    h.run();
    assert!(h.state().editor_close.is_none());
    assert_eq!(h.state().editors.len(), 1);

    h.get_by_role_and_label(Role::Window, &dirty_title)
        .get_by_label("Close window")
        .click_accesskit();
    h.run();
    h.get_by_role_and_label(Role::Window, "Unsaved changes")
        .get_by_label("Discard")
        .click_accesskit();
    h.run();
    assert!(h.state().editors.is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
    cleanup(&path);
}

#[test]
fn unsaved_editor_save_and_close_writes_the_file() {
    let path = temp_path("editor-save-close.lisp");
    std::fs::write(&path, "original").unwrap();
    let mut h = inproc_harness();
    h.run();
    h.state_mut().open_editor_path(path.clone());
    {
        let app = h.state_mut();
        app.editors[0].text = "saved on close".into();
        app.editors[0].dirty = true;
    }
    h.run();

    h.get_by_role_and_label(Role::Window, &format!("{} *", editor_title(&path)))
        .get_by_label("Close window")
        .click_accesskit();
    h.run();
    h.get_by_role_and_label(Role::Window, "Unsaved changes")
        .get_by_label("Save & Close")
        .click_accesskit();
    h.run();

    assert!(h.state().editors.is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "saved on close");
    cleanup(&path);
}

#[test]
fn opening_an_existing_editor_path_reuses_it() {
    let path = temp_path("editor-reuse.lisp");
    std::fs::write(&path, "source").unwrap();
    let mut h = inproc_harness();
    h.run();
    {
        let app = h.state_mut();
        app.open_editor_path(path.clone());
        app.open_editor_path(path.clone());
    }
    h.run();
    assert_eq!(h.state().editors.len(), 1);
    cleanup(&path);
}
