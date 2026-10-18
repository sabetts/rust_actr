// @generated then hand-maintained. Event-hook (`.lisp` `add-pre/post-event-hook`)
// cases, run through the IR in `tests/hooks/mod.rs`. This file is `include!`d by
// `tests/test_models.rs`; promote/generate via `tools/gen_hook_ir.py`.

// ---------------------------------------------------------------- unit2 demo2

const HOOK_DEMO2_ENCODE_LETTER_IMAGINAL_ACTIONS: hooks::HookIr = hooks::HookIr {
    driver: "demo2",
    rules: &[hooks::Rule {
        when: hooks::Pred::All(&[
            hooks::Pred::MsLt(450),
            hooks::Pred::Any(&[
                hooks::Pred::All(&[
                    hooks::Pred::Action("production-fired"),
                    hooks::Pred::Production("encode-letter"),
                ]),
                hooks::Pred::All(&[
                    hooks::Pred::Action("module-request"),
                    hooks::Pred::Buffer("imaginal"),
                ]),
                hooks::Pred::All(&[
                    hooks::Pred::Action("clear-buffer"),
                    hooks::Pred::Buffer("imaginal"),
                ]),
                hooks::Pred::Module("imaginal"),
            ]),
        ]),
        keep: true,
        set: None,
    }],
    else_dots: false,
    butlast: false,
    join: hooks::Join::NewlineDots,
};

#[test]
fn hook_tutorial_unit2_demo2_encode_letter_imaginal_actions() {
    if let Err(e) = hooks::run_hook_case(
        "tutorial-unit2-demo2-encode-letter-imaginal-actions",
        &HOOK_DEMO2_ENCODE_LETTER_IMAGINAL_ACTIONS,
    ) {
        panic!("{e}");
    }
}

const HOOK_DEMO2_ATTEND_LETTER_VISUAL_ACTIONS: hooks::HookIr = hooks::HookIr {
    driver: "demo2",
    rules: &[hooks::Rule {
        when: hooks::Pred::All(&[
            hooks::Pred::MsBetween(100, 185),
            hooks::Pred::Any(&[
                hooks::Pred::All(&[
                    hooks::Pred::Action("production-fired"),
                    hooks::Pred::Production("attend-letter"),
                ]),
                hooks::Pred::All(&[
                    hooks::Pred::Action("module-request"),
                    hooks::Pred::Buffer("visual"),
                ]),
                hooks::Pred::All(&[
                    hooks::Pred::Action("clear-buffer"),
                    hooks::Pred::Buffer("visual"),
                ]),
                hooks::Pred::Module("vision"),
            ]),
        ]),
        keep: true,
        set: None,
    }],
    else_dots: true,
    butlast: true,
    join: hooks::Join::Newline,
};

#[test]
fn hook_tutorial_unit2_demo2_attend_letter_visual_actions() {
    if let Err(e) = hooks::run_hook_case(
        "tutorial-unit2-demo2-attend-letter-visual-actions",
        &HOOK_DEMO2_ATTEND_LETTER_VISUAL_ACTIONS,
    ) {
        panic!("{e}");
    }
}

const HOOK_DEMO2_STRICT_HARVEST: hooks::HookIr = hooks::HookIr {
    driver: "demo2",
    rules: &[hooks::Rule {
        when: hooks::Pred::Any(&[
            hooks::Pred::All(&[
                hooks::Pred::Action("production-fired"),
                hooks::Pred::ProductionIn(&["attend-letter", "encode-letter", "respond"]),
            ]),
            hooks::Pred::All(&[
                hooks::Pred::Action("clear-buffer"),
                hooks::Pred::Any(&[
                    hooks::Pred::All(&[
                        hooks::Pred::MsEq(100),
                        hooks::Pred::Buffer("visual-location"),
                    ]),
                    hooks::Pred::All(&[hooks::Pred::MsEq(235), hooks::Pred::Buffer("visual")]),
                    hooks::Pred::All(&[hooks::Pred::MsEq(485), hooks::Pred::Buffer("imaginal")]),
                ]),
            ]),
        ]),
        keep: true,
        set: None,
    }],
    else_dots: true,
    butlast: true,
    join: hooks::Join::Newline,
};

#[test]
fn hook_tutorial_unit2_demo2_strict_harvest() {
    if let Err(e) = hooks::run_hook_case(
        "tutorial-unit2-demo2-strict-harvest",
        &HOOK_DEMO2_STRICT_HARVEST,
    ) {
        panic!("{e}");
    }
}

const HOOK_DEMO2_VISUAL_REENCODE_ACTIONS: hooks::HookIr = hooks::HookIr {
    driver: "demo2",
    rules: &[hooks::Rule {
        when: hooks::Pred::All(&[hooks::Pred::Module("vision"), hooks::Pred::MsEq(970)]),
        keep: true,
        set: None,
    }],
    else_dots: false,
    butlast: false,
    join: hooks::Join::Newline,
};

#[test]
fn hook_tutorial_unit2_demo2_visual_reencode_actions() {
    if let Err(e) = hooks::run_hook_case(
        "tutorial-unit2-demo2-visual-reencode-actions",
        &HOOK_DEMO2_VISUAL_REENCODE_ACTIONS,
    ) {
        panic!("{e}");
    }
}

const HOOK_DEMO2_FIND_UNATTENDED_LETTER_ACTIONS: hooks::HookIr = hooks::HookIr {
    driver: "demo2",
    rules: &[
        hooks::Rule {
            when: hooks::Pred::All(&[
                hooks::Pred::MsEq(50),
                hooks::Pred::Action("production-fired"),
                hooks::Pred::Production("find-unattended-letter"),
            ]),
            keep: true,
            set: Some(true),
        },
        hooks::Rule {
            when: hooks::Pred::All(&[
                hooks::Pred::MsEq(50),
                hooks::Pred::Latch,
                hooks::Pred::Action("conflict-resolution"),
            ]),
            keep: false,
            set: Some(false),
        },
        hooks::Rule {
            when: hooks::Pred::All(&[hooks::Pred::MsEq(50), hooks::Pred::Latch]),
            keep: true,
            set: None,
        },
    ],
    else_dots: false,
    butlast: false,
    join: hooks::Join::Newline,
};

#[test]
fn hook_tutorial_unit2_demo2_find_unattended_letter_actions() {
    if let Err(e) = hooks::run_hook_case(
        "tutorial-unit2-demo2-find-unattended-letter-actions",
        &HOOK_DEMO2_FIND_UNATTENDED_LETTER_ACTIONS,
    ) {
        panic!("{e}");
    }
}

const HOOK_DEMO2_OTHER_VISUAL_ACTIONS: hooks::HookIr = hooks::HookIr {
    driver: "demo2",
    rules: &[hooks::Rule {
        when: hooks::Pred::All(&[
            hooks::Pred::Module("vision"),
            hooks::Pred::MsIn(&[0, 885, 970]),
        ]),
        keep: true,
        set: None,
    }],
    else_dots: true,
    butlast: true,
    join: hooks::Join::Newline,
};

#[test]
fn hook_tutorial_unit2_demo2_other_visual_actions() {
    if let Err(e) = hooks::run_hook_case(
        "tutorial-unit2-demo2-other-visual-actions",
        &HOOK_DEMO2_OTHER_VISUAL_ACTIONS,
    ) {
        panic!("{e}");
    }
}

const HOOK_UNIT2CODE_DEMO2_MANUAL_ACTIONS_TRACE: hooks::HookIr = hooks::HookIr {
    driver: "demo2",
    rules: &[hooks::Rule {
        when: hooks::Pred::All(&[
            hooks::Pred::MsBetween(485, 895),
            hooks::Pred::Not(&hooks::Pred::Module("vision")),
        ]),
        keep: true,
        set: None,
    }],
    else_dots: false,
    butlast: true,
    join: hooks::Join::Newline,
};

#[test]
fn hook_tutorial_unit2code_demo2_manual_actions_trace() {
    if let Err(e) = hooks::run_hook_case(
        "tutorial-unit2code-demo2-manual-actions-trace",
        &HOOK_UNIT2CODE_DEMO2_MANUAL_ACTIONS_TRACE,
    ) {
        panic!("{e}");
    }
}

const HOOK_DEMO2_RESPOND_MANUAL_ACTIONS: hooks::HookIr = hooks::HookIr {
    driver: "demo2",
    rules: &[hooks::Rule {
        when: hooks::Pred::Any(&[
            hooks::Pred::All(&[
                hooks::Pred::Action("production-fired"),
                hooks::Pred::Production("respond"),
            ]),
            hooks::Pred::All(&[
                hooks::Pred::Action("module-request"),
                hooks::Pred::Buffer("manual"),
            ]),
            hooks::Pred::All(&[
                hooks::Pred::Action("clear-buffer"),
                hooks::Pred::Buffer("manual"),
            ]),
            hooks::Pred::Module("motor"),
            hooks::Pred::Module("keyboard"),
        ]),
        keep: true,
        set: None,
    }],
    else_dots: true,
    butlast: true,
    join: hooks::Join::Newline,
};

#[test]
fn hook_tutorial_unit2_demo2_respond_manual_actions() {
    if let Err(e) = hooks::run_hook_case(
        "tutorial-unit2-demo2-respond-manual-actions",
        &HOOK_DEMO2_RESPOND_MANUAL_ACTIONS,
    ) {
        panic!("{e}");
    }
}
