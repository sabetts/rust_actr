//! Manual/touch/keyboard motor (extended motor actions) and tracker.
use super::eval::{lisp_num_str, name_of, num_of, num_value};
use super::*;

pub(super) fn key_table() -> HashMap<String, KeySpec> {
    HashMap::from([
        (
            "Q".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "PINKIE",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "0".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "PINKIE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "1".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "PINKIE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "2".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "RING",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "3".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "MIDDLE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "4".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "5".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 2.24,
                theta: -1.11,
            },
        ),
        (
            "6".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 2.24,
                theta: -2.03,
            },
        ),
        (
            "7".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "8".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "MIDDLE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "9".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 2.0,
                theta: -1.57,
            },
        ),
        // QWERTY letter rows (`other-files/keyboard.lisp`). Punch-style
        // home keys (A S D F J K L) have no explicit polar geometry; use
        // the home position as a stand-in.
        (
            "Q".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "PINKIE",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "W".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "RING",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "E".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "MIDDLE",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "R".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "T".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 1.41,
                theta: -0.79,
            },
        ),
        (
            "Y".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 1.41,
                theta: -2.36,
            },
        ),
        (
            "U".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "I".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "MIDDLE",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "O".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "P".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "PINKIE",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "A".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "PINKIE",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "S".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "RING",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "D".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "MIDDLE",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "F".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "G".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "H".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 1.0,
                theta: 3.14,
            },
        ),
        (
            "J".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "K".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "MIDDLE",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "L".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "Z".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "PINKIE",
                r: 1.0,
                theta: 1.57,
            },
        ),
        (
            "X".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "RING",
                r: 1.0,
                theta: 1.57,
            },
        ),
        (
            "C".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "MIDDLE",
                r: 1.0,
                theta: 1.57,
            },
        ),
        (
            "V".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 1.0,
                theta: 1.57,
            },
        ),
        (
            "B".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 1.41,
                theta: 0.79,
            },
        ),
        (
            "N".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 1.41,
                theta: 2.36,
            },
        ),
        (
            "M".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 1.0,
                theta: 1.57,
            },
        ),
        (
            "SPACE".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "THUMB",
                r: 1.0,
                theta: 0.0,
            },
        ),
        (
            "CLEAR".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-=".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "MIDDLE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-/".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-*".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "PINKIE",
                r: 3.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-7".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-8".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "MIDDLE",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-9".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-MINUS".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "PINKIE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-4".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 0.0,
                theta: 0.0,
            },
        ),
        (
            "KEYPAD-5".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "MIDDLE",
                r: 0.0,
                theta: 0.0,
            },
        ),
        (
            "KEYPAD-6".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 0.0,
                theta: 0.0,
            },
        ),
        (
            "KEYPAD-PLUS".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "PINKIE",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "KEYPAD-1".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 1.0,
                theta: 1.57,
            },
        ),
        (
            "KEYPAD-2".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "MIDDLE",
                r: 1.0,
                theta: 1.57,
            },
        ),
        (
            "KEYPAD-3".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 1.0,
                theta: 1.57,
            },
        ),
        (
            "ENTER".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "PINKIE",
                r: 0.0,
                theta: 0.0,
            },
        ),
        (
            "KEYPAD-0".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "THUMB",
                r: 0.0,
                theta: 0.0,
            },
        ),
        (
            "KEYPAD-PERIOD".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 2.0,
                theta: 1.57,
            },
        ),
    ])
}

/// ACT-R 7's built-in manual command chunk types: the `defStyle`s in
/// `core-modules/motor.lisp` (`punch`, `peck`, `peck-recoil`, `hold-punch`,
/// `hold-peck`, `point-finger`, `release`, `hand-to-home`) plus the mouse and
/// keypad styles the virtual devices use (`move-cursor`, `click-mouse`,
/// `hold-mouse`, `release-mouse`).
///
/// These are **not** experimental. `extended-motor-actions` adds *further*
/// commands on top (`hold-key`, `type-key`, `prepare`/`execute`,
/// `delayed-punch`, …), which stay behind that flag — so a request naming one
/// of these must resolve with or without the extra module installed, or a
/// plain tutorial model (`bst`'s `move-cursor`, `keypad-test`'s `punch`)
/// cannot run at all.
pub(super) const STANDARD_MANUAL_CMDS: &[&str] = &[
    "PUNCH",
    "PECK",
    "PECK-RECOIL",
    "HOLD-PUNCH",
    "HOLD-PECK",
    "POINT-FINGER",
    "RELEASE",
    "HAND-TO-HOME",
    "MOVE-CURSOR",
    "CLICK-MOUSE",
    "HOLD-MOUSE",
    "RELEASE-MOUSE",
];

/// The slots a built-in style's chunk carries (`defStyle`): the movement
/// itself (`hand`/`finger`/`r`/`theta`) and, for the cursor styles, the target
/// location (`loc`).
pub(super) const STANDARD_MANUAL_SLOTS: &[&str] =
    &["HAND", "FINGER", "LOC", "R", "THETA", "TO-KEY", "OFFSETS"];

/// Keys whose `key->cmd` mapping is a `punch` movement (`other-files/keyboard.lisp`
/// ASDF row and space): the finger is already home, so no polar geometry is
/// prepared. Everything else is a `peck-recoil` with hand/finger/r/theta.
/// Punch prepares 3 features (hand, finger, and the implicit `+1`), peck 5.
pub(super) fn key_is_punch(key: &str) -> bool {
    matches!(
        key.to_uppercase().as_str(),
        "A" | "S"
            | "D"
            | "F"
            | "J"
            | "K"
            | "L"
            | ";"
            | "SPACE"
            | "KEYPAD-4"
            | "KEYPAD-5"
            | "KEYPAD-6"
            | "ENTER"
            | "KEYPAD-0"
    )
}

/// A key's absolute `(x, y)` in the virtual keyboard grid
/// (`other-files/keyboard.lisp` `specify-key`).
pub(super) fn key_xy(key: &str) -> Option<(i64, i64)> {
    let p = match key.to_uppercase().as_str() {
        "`" | "BACKQUOTE" => (0, 2),
        "1" => (1, 2),
        "2" => (2, 2),
        "3" => (3, 2),
        "4" => (4, 2),
        "5" => (5, 2),
        "6" => (6, 2),
        "7" => (7, 2),
        "8" => (8, 2),
        "9" => (9, 2),
        "0" => (10, 2),
        "-" | "HYPHEN" => (11, 2),
        "=" | "EQUAL" => (12, 2),
        "DELETE" => (13, 2),
        "TAB" => (0, 3),
        "Q" => (1, 3),
        "W" => (2, 3),
        "E" => (3, 3),
        "R" => (4, 3),
        "T" => (5, 3),
        "Y" => (6, 3),
        "U" => (7, 3),
        "I" => (8, 3),
        "O" => (9, 3),
        "P" => (10, 3),
        "[" | "LEFT-BRACKET" => (11, 3),
        "]" | "RIGHT-BRACKET" => (12, 3),
        "\\" | "BACKSLASH" => (13, 3),
        "CAPS-LOCK" => (0, 4),
        "A" => (1, 4),
        "S" => (2, 4),
        "D" => (3, 4),
        "F" => (4, 4),
        "G" => (5, 4),
        "H" => (6, 4),
        "J" => (7, 4),
        "K" => (8, 4),
        "L" => (9, 4),
        ";" | "SEMICOLON" => (10, 4),
        "'" | "QUOTE" => (11, 4),
        "RETURN" | "NEWLINE" => (12, 4),
        "SHIFT" | "LEFT-SHIFT" => (0, 5),
        "Z" => (1, 5),
        "X" => (2, 5),
        "C" => (3, 5),
        "V" => (4, 5),
        "B" => (5, 5),
        "N" => (6, 5),
        "M" => (7, 5),
        "," | "COMMA" => (8, 5),
        "." | "PERIOD" | "DOT" => (9, 5),
        "/" | "SLASH" => (10, 5),
        "RIGHT-SHIFT" => (11, 5),
        "SPACE" => (5, 6),
        "CLEAR" => (19, 2),
        "KEYPAD-=" => (20, 2),
        "KEYPAD-/" => (21, 2),
        "KEYPAD-*" => (22, 2),
        "KEYPAD-7" => (19, 3),
        "KEYPAD-8" => (20, 3),
        "KEYPAD-9" => (21, 3),
        "KEYPAD-MINUS" => (22, 3),
        "KEYPAD-4" => (19, 4),
        "KEYPAD-5" => (20, 4),
        "KEYPAD-6" => (21, 4),
        "KEYPAD-PLUS" => (22, 4),
        "KEYPAD-1" => (19, 5),
        "KEYPAD-2" => (20, 5),
        "KEYPAD-3" => (21, 5),
        "ENTER" => (22, 5),
        "KEYPAD-0" => (19, 6),
        "KEYPAD-PERIOD" => (21, 6),
        _ => return None,
    };
    Some(p)
}

/// `key->hand&finger` (`keyboard.lisp` `key->cmd` → the style's hand/finger).
/// A key's canonical lowercase name at absolute `(x, y)` — the inverse of
/// [`key_xy`], used to name the device's `output-key` from the finger position
/// (`motor.lisp:960`).
pub(super) fn xy_key(x: i64, y: i64) -> Option<&'static str> {
    let s = match (x, y) {
        (1, 2) => "1",
        (2, 2) => "2",
        (3, 2) => "3",
        (4, 2) => "4",
        (5, 2) => "5",
        (6, 2) => "6",
        (7, 2) => "7",
        (8, 2) => "8",
        (9, 2) => "9",
        (10, 2) => "0",
        (1, 3) => "q",
        (2, 3) => "w",
        (3, 3) => "e",
        (4, 3) => "r",
        (5, 3) => "t",
        (6, 3) => "y",
        (7, 3) => "u",
        (8, 3) => "i",
        (9, 3) => "o",
        (10, 3) => "p",
        (0, 2) => "`",
        (11, 2) => "-",
        (12, 2) => "=",
        (13, 2) => "delete",
        (0, 3) => "tab",
        (11, 3) => "[",
        (12, 3) => "]",
        (13, 3) => "\\",
        (0, 4) => "caps-lock",
        (1, 4) => "a",
        (2, 4) => "s",
        (3, 4) => "d",
        (4, 4) => "f",
        (5, 4) => "g",
        (6, 4) => "h",
        (7, 4) => "j",
        (8, 4) => "k",
        (9, 4) => "l",
        (1, 5) => "z",
        (2, 5) => "x",
        (3, 5) => "c",
        (4, 5) => "v",
        (5, 5) => "b",
        (6, 5) => "n",
        (7, 5) => "m",
        (8, 5) => ",",
        (9, 5) => ".",
        (10, 5) => "/",
        (11, 5) => "right-shift",
        (10, 4) => ";",
        (11, 4) => "'",
        (12, 4) => "return",
        (0, 5) => "shift",
        (5, 6) => "space",
        (6, 6) => "space",
        (19, 2) => "clear",
        (20, 2) => "keypad-=",
        (21, 2) => "keypad-/",
        (22, 2) => "keypad-*",
        (19, 3) => "keypad-7",
        (20, 3) => "keypad-8",
        (21, 3) => "keypad-9",
        (22, 3) => "keypad--",
        (19, 4) => "keypad-4",
        (20, 4) => "keypad-5",
        (21, 4) => "keypad-6",
        (22, 4) => "keypad-+",
        (19, 5) => "keypad-1",
        (20, 5) => "keypad-2",
        (21, 5) => "keypad-3",
        (22, 5) => "enter",
        (19, 6) => "keypad-0",
        (21, 6) => "keypad-.",
        _ => return None,
    };
    Some(s)
}

/// Keyboard aliases report the canonical device key from `specify-key`.
pub(super) fn canonical_output_key(key: &str) -> String {
    match key.to_uppercase().as_str() {
        "KEYPAD-" | "KEYPAD--" | "KEYPAD-MINUS" | "KEYPAD-HYPHEN" => "keypad--".to_string(),
        "KEYPAD-+" | "KEYPAD-PLUS" => "keypad-+".to_string(),
        "KEYPAD-." | "KEYPAD-PERIOD" | "KEYPAD-DOT" => "keypad-.".to_string(),
        "KEYPAD-=" | "KEYPAD-EQUAL" => "keypad-=".to_string(),
        "KEYPAD-/" | "KEYPAD-DIVIDE" | "KEYPAD-SLASH" => "keypad-/".to_string(),
        "KEYPAD-*" | "KEYPAD-ASTERISK" | "KEYPAD-TIMES" => "keypad-*".to_string(),
        _ => key.to_lowercase(),
    }
}

pub(super) fn key_hand_finger(key: &str) -> Option<(usize, usize)> {
    let (hand, finger) = match key.to_uppercase().as_str() {
        "`" | "BACKQUOTE" | "1" | "Q" | "A" | "Z" | "TAB" | "CAPS-LOCK" | "SHIFT"
        | "LEFT-SHIFT" => (0usize, 3usize),
        "2" | "W" | "S" | "X" => (0, 2),
        "3" | "E" | "D" | "C" => (0, 1),
        "4" | "R" | "F" | "V" | "5" | "T" | "G" | "B" => (0, 0),
        "6" | "Y" | "H" | "N" => (1, 0),
        "7" | "U" | "J" | "M" => (1, 0),
        "8" | "I" | "K" | "," | "COMMA" => (1, 1),
        "9" | "O" | "L" | "." | "PERIOD" | "DOT" => (1, 2),
        "0" | "P" | ";" | "SEMICOLON" | "/" | "SLASH" | "-" | "HYPHEN" | "=" | "EQUAL" | "["
        | "LEFT-BRACKET" | "]" | "RIGHT-BRACKET" | "\\" | "BACKSLASH" | "'" | "QUOTE"
        | "DELETE" | "RETURN" | "NEWLINE" | "RIGHT-SHIFT" => (1, 3),
        "SPACE" => (0, 4),
        _ => return None,
    };
    Some((hand, finger))
}

impl Engine {
    /// `style X hand Y` suffix for motor events when the extended-motor-actions
    /// module is loaded (`motor-extension.lisp:979`).
    pub(super) fn zz_style_suffix(&self) -> String {
        if !self.cfg.zz_ext {
            return String::new();
        }
        match &self.zz_last_style {
            Some((class, _, hand)) => format!(" style {class} hand {hand}"),
            None => String::new(),
        }
    }

    /// `style X hand Y` suffix from an explicit `(style, hand)`.
    pub(super) fn zz_style_suffix_for(&self, style: &str, hand: usize) -> String {
        self.zz_style_suffix_hand(style, hand, false)
    }

    /// `style X hand Y` suffix, with `BOTH` for the two-handed styles
    /// (`all-fingers-to-home`).
    pub(super) fn zz_style_suffix_hand(&self, style: &str, hand: usize, both: bool) -> String {
        if !self.cfg.zz_ext {
            return String::new();
        }
        let h = if both {
            "BOTH"
        } else if hand == 0 {
            "LEFT"
        } else {
            "RIGHT"
        };
        format!(" style {style} hand {h}")
    }
}

impl Engine {
    /// act-touch: hand index (left=0, right=1).
    pub(super) fn touch_hand_idx(hand: &str) -> Option<usize> {
        match hand.to_uppercase().as_str() {
            "LEFT" => Some(0),
            "RIGHT" => Some(1),
            _ => None,
        }
    }

    /// act-touch: finger index (`index middle ring pinkie thumb`).
    pub(super) fn touch_finger_idx(finger: &str) -> Option<usize> {
        match finger.to_uppercase().as_str() {
            "INDEX" => Some(0),
            "MIDDLE" => Some(1),
            "RING" => Some(2),
            "PINKIE" => Some(3),
            "THUMB" => Some(4),
            _ => None,
        }
    }

    pub(super) fn touch_any_finger_down(&self, h: usize) -> bool {
        self.touch_fingers[h].iter().any(|f| f.2 == 0)
    }

    /// `(start-hand-at-touch [hand])`: put a hand on the touch device at its
    /// configured start position (`act-touch.lisp:500`). The standard finger
    /// offsets (`motor.lisp:508`) place the index finger at the start.
    pub fn start_hand_at_touch(&mut self, hand: &str) {
        let Some(h) = Self::touch_hand_idx(hand) else {
            return;
        };
        let (sx, sy) = if h == 0 {
            self.cfg.touch_left_start
        } else {
            self.cfg.touch_right_start
        };
        let dist = self.cfg.touch_ppi * self.cfg.touch_finger_spacing;
        // (index, middle, ring, pinkie, thumb) standard offsets, mirrored
        // for the left hand.
        let offs: [(f32, f32); 5] = if h == 0 {
            [
                (0.0, 0.0),
                (-1.0, 0.0),
                (-2.0, 0.0),
                (-3.0, 0.0),
                (1.0, 2.0),
            ]
        } else {
            [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0), (-1.0, 2.0)]
        };
        for (i, (dx, dy)) in offs.iter().enumerate() {
            self.touch_fingers[h][i] = (
                (sx as f32 + dx * dist).round() as i64,
                (sy as f32 + dy * dist).round() as i64,
                1,
            );
        }
        self.touch_hands[h] = true;
    }

    /// A touch movement request (`act-touch.lisp`): compute the style's
    /// prep/exec/finish and schedule the device signal, monitor output and
    /// finger state changes.
    pub(super) fn touch_request(&mut self, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let mut cmd = String::new();
        let mut hand = "RIGHT".to_string();
        let mut finger = "INDEX".to_string();
        let mut loc: Option<(i64, i64, String)> = None;
        let mut object_given = false;
        let mut r_req: Option<f64> = None;
        let mut theta_req: Option<f64> = None;
        let mut num_fngrs: i64 = 1;
        let mut swipe_speed: Option<i64> = None;
        let mut start_width = 0.0f32;
        let mut end_width = 0.0f32;
        let mut rotation = 0.0f32;
        let mut disp: Vec<(String, String)> = Vec::new();
        for (s, v) in slots {
            disp.push((s.clone(), v.show()));
            match s.as_str() {
                "CMD" => {
                    if let Value::Sym(c) = v {
                        cmd = c.to_uppercase();
                    }
                }
                "HAND" => hand = name_of(v),
                "FINGER" => finger = name_of(v),
                "LOC" => loc = self.chunk_xy_name(v),
                "OBJECT" => {
                    object_given = true;
                    if loc.is_none() {
                        loc = self.chunk_xy_name(v);
                    }
                }
                "R" => r_req = num_of(v),
                "THETA" => theta_req = num_of(v),
                "NUM-FNGRS" => num_fngrs = num_of(v).unwrap_or(1.0) as i64,
                "SWIPE-SPEED" => swipe_speed = num_of(v).map(|x| x as i64),
                "START-WIDTH" => start_width = num_of(v).unwrap_or(0.0) as f32,
                "END-WIDTH" => end_width = num_of(v).unwrap_or(0.0) as f32,
                "ROTATION" => rotation = num_of(v).unwrap_or(0.0) as f32,
                _ => {}
            }
        }
        let h = Self::touch_hand_idx(&hand).ok_or_else(|| unimpl(format!("touch hand {hand}")))?;
        let init = self.cfg.init_ms;
        let burst = self.cfg.burst_ms;
        let feat = self.cfg.feat_prep_ms;
        let model = self.model.name.to_uppercase();
        let min_fitts = self.cfg.min_fitts_ms as f32 / 1000.0;

        // Style name + fresh feature count + feat-differences parameters.
        let (style, fresh, nf) = match cmd.as_str() {
            "MOVE-HAND-TOUCH" => ("TOUCH-PLY", 4i64, 4i64),
            "TAP" => ("TAP", 3, 3),
            "TAP-HOLD" => ("TAP-HOLD", 3, 3),
            "TAP-RELEASE" => ("TAP-RELEASE", 3, 3),
            "SWIPE" => ("SWIPE", 7, 7),
            "PINCH" => ("PINCH", 5, 5),
            "ROTATE" => ("ROTATE", 4, 4),
            "HAND-TO-TOUCH" => ("HAND-TO-TOUCH", 4, 4),
            other => return Err(unimpl(format!("touch command {other}"))),
        };
        let feats = match &self.touch_last {
            None => fresh,
            Some((ls, lh, lf, lr, lt)) if ls == style => {
                if lh != &hand {
                    nf - 1
                } else if lf != &finger {
                    nf - 2
                } else {
                    // `rotate`/`pinch` leave r/theta unset (`nil`), so
                    // `distance=`/`direction=` report both as different;
                    // `tap` has no r/theta slots at all.
                    match style {
                        "ROTATE" | "PINCH" => 2,
                        "TAP" | "TAP-HOLD" | "TAP-RELEASE" => 0,
                        _ => {
                            let mut n = 0;
                            if (*lr - r_req.unwrap_or(0.0) as f32).abs() > 1e-4 {
                                n += 1;
                            }
                            if (*lt - theta_req.unwrap_or(0.0) as f32).abs() > 1e-4 {
                                n += 1;
                            }
                            n
                        }
                    }
                }
            }
            Some(_) => fresh,
        };
        self.touch_last = Some((
            style.to_string(),
            hand.clone(),
            finger.clone(),
            r_req.unwrap_or(0.0) as f32,
            theta_req.unwrap_or(0.0) as f32,
        ));
        let prep_ms = feats * feat;
        let _ = object_given;

        // Per-style exec/finish and device signal.
        let mut plan = TouchPlan {
            req_ms,
            prep_ms,
            init_ms: init,
            exec_ms: 0,
            finish_ms: 0,
            signal: String::new(),
            signal_module: "TOUCH-INTERFACE".to_string(),
            monitor: String::new(),
            hand: hand.clone(),
            finger: finger.clone(),
            down_at_init: false,
            down_at_exec: false,
            up_at_exec: false,
            up_at_exec_burst: false,
            move_to: None,
            put_hand: false,
        };
        let fitts =
            |coef: f32, d: f32, w: f32| -> f32 { (coef * (d / w + 0.5).log2()).max(min_fitts) };
        let get = |k: &str| -> String {
            disp.iter()
                .find(|(s, _)| s == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        match cmd.as_str() {
            "MOVE-HAND-TOUCH" => {
                let Some((tx, ty, _lname)) = loc else {
                    return Err(unimpl("move-hand-touch without a location"));
                };
                let (cx, cy) = (self.touch_fingers[h][0].0, self.touch_fingers[h][0].1);
                let dx = (tx - cx) as f32;
                let dy = (ty - cy) as f32;
                let r = (dx * dx + dy * dy).sqrt();
                let theta = dy.atan2(dx);
                // `approach-width` (`vision.lisp:3717`): use the loc's
                // width/height if present, else the 1° default.
                let (fw, fh) = self.loc_wh(&_lname);
                let w = if fw > 0 && fh > 0 {
                    let (x, y) = (fw as f32, fh as f32);
                    let critical = (y / x).atan();
                    let mut th = theta.abs();
                    if th > std::f32::consts::FRAC_PI_2 {
                        th = std::f32::consts::PI - th;
                    }
                    let ret = if th == 0.0 {
                        x
                    } else if th == std::f32::consts::FRAC_PI_2 {
                        y
                    } else if th <= critical {
                        x / th.cos()
                    } else {
                        y / (std::f32::consts::FRAC_PI_2 - th).cos()
                    };
                    ret.round()
                } else {
                    // `approach-width` default (1°) → `pm-angle-to-pixels`
                    // rounds to 19 px.
                    19.0
                };
                let mut coef = self.cfg.touch_cursor_coeff;
                if self.cfg.touch_drag_delay && self.touch_any_finger_down(h) {
                    coef *= 2.0f32.powf(0.8);
                }
                let t = fitts(coef, r, w);
                plan.exec_ms = init + (t * 1000.0).round() as i64;
                plan.finish_ms = plan.exec_ms + burst;
                plan.signal = format!("touch-point {model} {hand} {tx} {ty}");
                plan.monitor = format!("model {model} moved the {hand} hand to point {tx} , {ty}");
                plan.move_to = Some((tx, ty));
            }
            "TAP" | "TAP-HOLD" => {
                let angle = (2.0f32
                    * ((self.cfg.touch_ppi * self.cfg.touch_height)
                        / 2.0
                        / (self.cfg.touch_ppi * 15.0))
                        .atan())
                .to_degrees();
                let t = fitts(self.cfg.touch_peck_coeff, angle, 1.0);
                plan.exec_ms = init + (burst as f32).max(t * 1000.0).round() as i64;
                plan.finish_ms = plan.exec_ms + burst;
                let (fx, fy) = {
                    let fi = Self::touch_finger_idx(&finger).unwrap_or(0);
                    (self.touch_fingers[h][fi].0, self.touch_fingers[h][fi].1)
                };
                plan.down_at_exec = true;
                if cmd == "TAP" {
                    plan.up_at_exec_burst = true;
                }
                let sig = if cmd == "TAP" {
                    "touch-tap"
                } else {
                    "touch-tap-hold"
                };
                plan.signal = format!("{sig} {model} {hand} {finger} {fx} {fy}");
                let verb = if cmd == "TAP" {
                    format!("tapped the {finger} finger on the {hand} hand at point {fx} , {fy}")
                } else {
                    format!("tapped and held the {finger} finger on the {hand} hand at point {fx} , {fy}")
                };
                plan.monitor = format!("model {model} {verb}");
            }
            "TAP-RELEASE" => {
                plan.exec_ms = init + burst;
                plan.finish_ms = plan.exec_ms + burst;
                let fi = Self::touch_finger_idx(&finger).unwrap_or(0);
                let (fx, fy) = (self.touch_fingers[h][fi].0, self.touch_fingers[h][fi].1);
                plan.up_at_exec = true;
                plan.signal = format!("touch-tap-release {model} {hand} {finger} {fx} {fy}");
                plan.monitor = format!(
                    "model {model} released the {finger} finger on the {hand} hand at point {fx} , {fy}"
                );
            }
            "SWIPE" => {
                let r = r_req.unwrap_or(0.0) as f32;
                let theta = theta_req.unwrap_or(0.0) as f32;
                let w = self.cfg.touch_ppi * self.cfg.touch_thumb_width;
                let mv = (burst as f32 / 1000.0).max(fitts(self.cfg.touch_peck_coeff, r, w));
                let exec = init as f32 / 1000.0 + (burst as f32 / 1000.0).max(mv);
                plan.exec_ms = (exec * 1000.0).round() as i64;
                plan.finish_ms = ((exec + burst as f32 / 1000.0 + mv) * 1000.0).round() as i64;
                plan.down_at_init = true;
                plan.up_at_exec_burst = true;
                let speed = swipe_speed.unwrap_or(self.cfg.touch_swipe_speed);
                let r_disp = get("R");
                let t_disp = get("THETA");
                plan.signal = format!(
                    "touch-swipe {model} {hand} {finger} {r_disp} {t_disp} {num_fngrs} {speed}"
                );
                plan.monitor = format!(
                    "model {model} swiped {num_fngrs} fingers on the {hand} hand starting with the {finger} finger a distance of {} in the {} direction at speed {}",
                    lisp_s_f32(r), lisp_s_f32(theta), lisp_s_f32(speed as f32)
                );
            }
            "PINCH" => {
                let mv = (burst as f32 / 1000.0).max(fitts(
                    self.cfg.touch_peck_coeff,
                    (start_width - end_width).abs(),
                    self.cfg.touch_ppi * self.cfg.touch_thumb_width,
                ));
                let exec = init as f32 / 1000.0 + (burst as f32 / 1000.0).max(mv);
                plan.exec_ms = (exec * 1000.0).round() as i64;
                plan.finish_ms = ((exec + burst as f32 / 1000.0 + mv) * 1000.0).round() as i64;
                plan.down_at_init = true;
                plan.up_at_exec_burst = true;
                plan.signal = format!(
                    "touch-pinch {model} {hand} {finger} {} {}",
                    start_width as i64, end_width as i64
                );
                plan.monitor = format!(
                    "model {model} pinched with the {finger} finger on the {hand} hand from a width of {} to {}",
                    lisp_s_f32(start_width), lisp_s_f32(end_width)
                );
            }
            "ROTATE" => {
                let diameter = 54.0f32 * 5.0f32.sqrt();
                let w = 0.5
                    * diameter
                    * std::f32::consts::PI
                    * (1.0 / 180.0)
                    * self.cfg.touch_rotate_width;
                let mv = (burst as f32 / 1000.0).max(fitts(
                    self.cfg.touch_peck_coeff,
                    0.5 * diameter * rotation.abs(),
                    w,
                ));
                let exec = init as f32 / 1000.0 + (burst as f32 / 1000.0).max(mv);
                plan.exec_ms = (exec * 1000.0).round() as i64;
                plan.finish_ms = ((exec + burst as f32 / 1000.0 + mv) * 1000.0).round() as i64;
                plan.down_at_init = true;
                plan.up_at_exec_burst = true;
                plan.signal = format!(
                    "touch-rotate {model} {hand} {finger} {}",
                    lisp_s_f32(rotation)
                );
                plan.monitor = format!(
                    "model {model} rotated with the {finger} finger on the {hand} hand  {} radians",
                    lisp_s_f32(rotation)
                );
            }
            "HAND-TO-TOUCH" => {
                // to-touch-ply (hand-ply): exec = init + fitts(mouse, r, twidth).
                let r = 5.3851647f32;
                let theta = 1.19029f32;
                let t = fitts(self.cfg.touch_cursor_coeff, r, 4.0);
                plan.exec_ms = init + (t * 1000.0).round() as i64;
                plan.finish_ms = plan.exec_ms + burst;
                plan.signal_module = "MOTOR".to_string();
                plan.signal = format!("MOVE-A-HAND {hand} {} {}", lisp_s_f32(r), lisp_s_f32(theta));
                plan.put_hand = true;
            }
            _ => unreachable!(),
        }
        let trace = match cmd.as_str() {
            "MOVE-HAND-TOUCH" => {
                format!("MOVE-HAND-TOUCH OBJECT NIL LOC {} HAND {hand}", get("LOC"))
            }
            "TAP" | "TAP-HOLD" | "TAP-RELEASE" => {
                format!("{cmd} HAND {hand} FINGER {finger}")
            }
            "PINCH" => format!(
                "PINCH HAND {hand} FINGER {finger} START-WIDTH {} END-WIDTH {}",
                get("START-WIDTH"),
                get("END-WIDTH")
            ),
            "ROTATE" => format!(
                "ROTATE HAND {hand} FINGER {finger} ROTATION {}",
                get("ROTATION")
            ),
            "SWIPE" => {
                let mut s = format!(
                    "SWIPE HAND {hand} FINGER {finger} R {} THETA {} NUM-FNGRS {}",
                    get("R"),
                    get("THETA"),
                    get("NUM-FNGRS")
                );
                if disp.iter().any(|(k, _)| k == "SWIPE-SPEED") {
                    s.push_str(&format!(" SWIPE-SPEED {}", get("SWIPE-SPEED")));
                }
                s
            }
            "HAND-TO-TOUCH" => "HAND-TO-TOUCH".to_string(),
            _ => String::new(),
        };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: trace,
            },
        );
        self.man_prep_busy = true;
        let at = self.time_ms + plan.prep_ms;
        self.push(
            at,
            Prio::Num(0),
            Out::Always,
            Event::TouchPrep {
                plan: Box::new(plan),
            },
        );
        Ok(())
    }

    /// tracker extra: choices uniformly spaced from `min` to `max`
    /// (`generate-tracker-choices`).
    pub(super) fn tracker_choices(n: usize, min: f64, max: f64) -> Vec<f64> {
        let interval = max - min;
        let step = interval / (n as f64 - 1.0);
        let mut out = Vec::new();
        let mut val = min;
        for i in 0..n {
            out.push(val);
            if i + 1 < n {
                val = max.min(val + step);
            }
        }
        out
    }

    /// tracker extra: initial experience statistics (`tracker-prior-statistics`).
    pub(super) fn tracker_prior_stats(choices: &[f64]) -> Vec<f64> {
        let mut s = vec![0.0f64; 11];
        for &v in choices {
            s[0] += 1.0;
            s[1] += v;
            s[2] += v * v;
            s[3] += v * v * v;
            s[4] += v * v * v * v;
        }
        s
    }

    /// tracker extra: `+tracker>` create request (`tracker.lisp:608`).
    pub(super) fn tracker_request(&mut self, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let get = |k: &str| slots.iter().find(|(s, _)| s == k).map(|(_, v)| v.clone());
        let name_of = |v: &Value| match v {
            Value::Sym(s) | Value::Str(s) => s.clone(),
            _ => String::new(),
        };
        let num_of = |v: &Value| match v {
            Value::Int(i) => Some(*i as f64),
            Value::Num(x) => Some(*x),
            _ => None,
        };
        let control_slot = get("CONTROL-SLOT").map(|v| name_of(&v)).unwrap_or_default();
        let good_slot = get("GOOD-SLOT").map(|v| name_of(&v));
        let bad_slot = get("BAD-SLOT").map(|v| name_of(&v));
        let min = get("MIN").and_then(|v| num_of(&v)).unwrap_or(0.0);
        let max = get("MAX").and_then(|v| num_of(&v)).unwrap_or(1.0);
        let count = get("CONTROL-COUNT")
            .and_then(|v| num_of(&v))
            .unwrap_or(21.0) as usize;
        let delay = get("DELAY").and_then(|v| num_of(&v)).unwrap_or(10.0);
        let temp = get("TEMP").and_then(|v| num_of(&v)).unwrap_or(1.0);
        let scale = get("SCALE").and_then(|v| num_of(&v)).unwrap_or(180.0);
        let good_weight = get("GOOD-WEIGHT").and_then(|v| num_of(&v)).unwrap_or(1.0);
        let bad_weight = get("BAD-WEIGHT").and_then(|v| num_of(&v)).unwrap_or(-1.0);
        let control_buffer = "GOAL".to_string();
        let values_buffer = "IMAGINAL".to_string();
        let now = self.time_ms;
        let n = *self.name_count.entry("TRACKER".to_string()).or_insert(0);
        self.name_count.insert("TRACKER".to_string(), n + 1);
        let name = format!("TRACKER{n}");
        let choices = Self::tracker_choices(count, min, max);
        let statistics = Self::tracker_prior_stats(&choices);
        let idx = self.trackers.len();
        self.trackers.push(Tracker {
            name,
            active: true,
            control_buffer: control_buffer.clone(),
            control_slot: control_slot.clone(),
            choices,
            statistics,
            params: [0.0; 3],
            init_temp: temp,
            temperature: temp,
            delay,
            scale_temp_ms: scale * 1000.0,
            active_time: 0,
            good_buffer: good_slot.as_ref().map(|_| values_buffer.clone()),
            good_slot,
            good_weight,
            good_value: 0.0,
            bad_buffer: bad_slot.as_ref().map(|_| values_buffer.clone()),
            bad_slot,
            bad_weight,
            bad_value: 0.0,
            last_update: now,
            setting: 0.0,
            update_gen: 0,
        });
        let setting = self.tracker_gen_quad_value(idx);
        self.trackers[idx].last_update = self.time_ms;
        self.trackers[idx].setting = setting;
        // `schedule-mod-buffer-chunk` at priority -2000, under TRACKER.
        self.now(
            Prio::Num(-2000),
            Out::Medium,
            Event::ModBuffer {
                buffer: control_buffer,
                slots: vec![(control_slot, num_value(setting))],
                module: "TRACKER".to_string(),
            },
        );
        self.tracker_schedule_update(idx, now);
        Ok(())
    }

    /// tracker extra: `create-update-tracker-event` (`tracker.lisp:578`).
    pub(super) fn tracker_schedule_update(&mut self, idx: usize, from_ms: i64) {
        let delay = self.trackers[idx].delay;
        let p = self.rng.borrow_mut().random_float(1.0) as f64;
        let t = (p.ln() * -delay).clamp(1.0, 60.0);
        let at = from_ms + (t * 1000.0).round() as i64;
        self.tracker_gen += 1;
        let gen = self.tracker_gen;
        self.trackers[idx].update_gen = gen;
        let name = self.trackers[idx].name.clone();
        let slot = self.trackers[idx].control_slot.clone();
        self.push(at, Prio::Num(0), Out::Always, Event::TrackerUpdate { gen });
        let _ = (name, slot);
    }

    /// tracker extra: Boltzmann choice + trace (`tracker-genQuadValue`).
    pub(super) fn tracker_gen_quad_value(&mut self, idx: usize) -> f64 {
        let choices = self.trackers[idx].choices.clone();
        let temperature = self.trackers[idx].temperature;
        let params = self.trackers[idx].params;
        let ys: Vec<f64> = choices
            .iter()
            .map(|&x| params[0] + x * params[1] + x * x * params[2])
            .collect();
        let max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let vals: Vec<f64> = ys
            .iter()
            .map(|&y| ((y - max) / temperature).exp())
            .collect();
        let sum: f64 = vals.iter().sum();
        let probs: Vec<f64> = vals.iter().map(|&v| v / sum).collect();
        let p = self.rng.borrow_mut().random_float(1.0) as f64;
        if self.cfg.trace_tracker {
            let name = self.trackers[idx].name.clone();
            let cb = self.trackers[idx].control_buffer.clone();
            let cs = self.trackers[idx].control_slot.clone();
            self.out_lines.push(format!(
                "Tracker {name} generating value for buffer {cb} slot {cs} with temperature {}:",
                lisp_s_f32(temperature as f32)
            ));
            let w = choices
                .iter()
                .map(|x| format!("{x:.3}").len())
                .max()
                .unwrap_or(5);
            let fmt = |xs: &[f64]| -> String {
                xs.iter()
                    .map(|x| format!("{x:>w$.3}", w = w))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            self.out_lines.push(format!("  Choices: {}", fmt(&choices)));
            self.out_lines.push(format!("  Probs:   {}", fmt(&probs)));
        }
        let mut result = *choices.last().unwrap();
        let mut cumsum = 0.0;
        for i in 0..choices.len() {
            cumsum += probs[i];
            if p < cumsum {
                result = choices[i];
                break;
            }
        }
        if self.cfg.trace_tracker {
            self.out_lines
                .push(format!(" Chosen result is {}", lisp_s_f32(result as f32)));
        }
        result
    }

    /// tracker extra: `update-tracker` (`tracker.lisp:1298`).
    pub(super) fn tracker_update(&mut self) -> Result<(), EngineError> {
        for idx in 0..self.trackers.len() {
            if !self.trackers[idx].active {
                continue;
            }
            self.tracker_update_stats(idx, false);
            self.tracker_calc_quad(idx);
            // `create-update-tracker-event` draws the next delay BEFORE
            // `genQuadValue` in `update-tracker` (`tracker.lisp:1303`).
            self.tracker_schedule_update(idx, self.time_ms);
            if self.cfg.trace_tracker {
                let name = self.trackers[idx].name.clone();
                let cs = self.trackers[idx].control_slot.clone();
                let p = self.trackers[idx].params;
                self.out_lines.push(format!(
                    "Tracker {name} for {cs} slot updating the equation"
                ));
                self.out_lines.push(format!(
                    "  new equation is {} + {}x + {}x^2",
                    lisp_s_f32(p[0] as f32),
                    lisp_s_f32(p[1] as f32),
                    lisp_s_f32(p[2] as f32)
                ));
            }
            let setting = self.tracker_gen_quad_value(idx);
            self.trackers[idx].setting = setting;
            self.trackers[idx].last_update = self.time_ms;
            let cb = self.trackers[idx].control_buffer.clone();
            let cs = self.trackers[idx].control_slot.clone();
            self.now(
                Prio::Num(-2000),
                Out::Medium,
                Event::ModBuffer {
                    buffer: cb,
                    slots: vec![(cs, num_value(setting))],
                    module: "TRACKER".to_string(),
                },
            );
        }
        Ok(())
    }

    /// tracker extra: `tracker-update-stats` (`tracker.lisp:1338`). ACT-R
    /// accumulates in single-float (`ms->seconds` returns one).
    pub(super) fn tracker_update_stats(&mut self, idx: usize, skip: bool) {
        let weight = ((self.time_ms - self.trackers[idx].last_update) as f32 / 1000.0).max(0.05);
        let weight2 = 1.0f32;
        let x = self.trackers[idx].setting as f32;
        let x2 = x * x;
        let good_y = self.trackers[idx].good_value as f32 / weight;
        let bad_y = self.trackers[idx].bad_value as f32 / weight;
        let current: [f32; 11] = [
            1.0,
            x,
            x2,
            x * x2,
            x2 * x2,
            good_y,
            x * good_y,
            x2 * good_y,
            bad_y,
            x * bad_y,
            x2 * bad_y,
        ];
        let old: Vec<f32> = self.trackers[idx]
            .statistics
            .iter()
            .map(|v| *v as f32)
            .collect();
        let new: Vec<f32> = if skip {
            old.iter().map(|o| weight2 * o).collect()
        } else {
            current
                .iter()
                .zip(old.iter())
                .map(|(c, o)| weight * c + weight2 * o)
                .collect()
        };
        self.trackers[idx].good_value = 0.0;
        self.trackers[idx].bad_value = 0.0;
        if !skip {
            self.trackers[idx].active_time += self.time_ms - self.trackers[idx].last_update;
            // ACT-R computes this as an exact rational; keep f64 precision
            // so the single-float coercion matches (`~f` in the trace).
            self.trackers[idx].temperature = self.trackers[idx].init_temp
                * self.trackers[idx].scale_temp_ms
                / (self.trackers[idx].scale_temp_ms + self.trackers[idx].active_time as f64);
        }
        self.trackers[idx].statistics = new.iter().map(|v| *v as f64).collect();
    }

    /// tracker extra: least-squares quadratic fit (`tracker-calc-quad`).
    pub(super) fn tracker_calc_quad(&mut self, idx: usize) {
        let s: Vec<f32> = self.trackers[idx]
            .statistics
            .iter()
            .map(|v| *v as f32)
            .collect();
        let wg = self.trackers[idx].good_weight as f32;
        let wb = self.trackers[idx].bad_weight as f32;
        let n = s[0];
        let (sx1, sx2, sx3, sx4) = (s[1], s[2], s[3], s[4]);
        let sy = wg * s[5] + wb * s[8];
        let sx1y = wg * s[6] + wb * s[9];
        let sx2y = wg * s[7] + wb * s[10];
        let lx2 = sx2 - sx1 * sx1 / n;
        let lx3 = sx3 - sx1 * sx2 / n;
        let lx4 = sx4 - sx2 * sx2 / n;
        let lx1y = sx1y - sx1 * sy / n;
        let lx2y = sx2y - sx2 * sy / n;
        let denom = lx2 * lx4 - lx3 * lx3;
        if denom == 0.0 {
            return;
        }
        let p2 = (lx4 * lx1y - lx3 * lx2y) / denom;
        let p3 = (lx2 * lx2y - lx3 * lx1y) / denom;
        let p1 = (sy - p2 * sx1 - p3 * sx2) / n;
        self.trackers[idx].params = [p1 as f64, p2 as f64, p3 as f64];
    }

    /// tracker extra: buffer-change watcher (`tracker-check-for-buffer-changes`).
    pub(super) fn tracker_check_buffer_changes(&mut self, buffer: &str, slot: Option<&str>) {
        if !self.cfg.tracker || self.trackers.is_empty() {
            return;
        }
        for idx in 0..self.trackers.len() {
            if !self.trackers[idx].active {
                continue;
            }
            let good = self.trackers[idx].good_buffer.as_deref() == Some(buffer);
            let bad = self.trackers[idx].bad_buffer.as_deref() == Some(buffer);
            if !good && !bad {
                continue;
            }
            let which_good = good
                && slot
                    .map(|s| Some(s) == self.trackers[idx].good_slot.as_deref())
                    .unwrap_or(false);
            let which_bad = bad
                && slot
                    .map(|s| Some(s) == self.trackers[idx].bad_slot.as_deref())
                    .unwrap_or(false);
            if !which_good && !which_bad {
                continue;
            }
            let name = self.trackers[idx].name.clone();
            let cs = self.trackers[idx].control_slot.clone();
            let which = if which_good { "GOOD" } else { "BAD" };
            if self.cfg.trace_tracker {
                self.out_lines.push(format!(
                    "Tracker {name} for {cs} updating the current {which} value"
                ));
                self.out_lines.push(format!(
                    " Updating event occurred in the {which} slot of the {buffer} buffer"
                ));
                self.out_lines.push("  updating with value 1.0".to_string());
            }
            if which_good {
                self.trackers[idx].good_value += 1.0;
            } else {
                self.trackers[idx].bad_value += 1.0;
            }
        }
    }

    /// tracker extra: the `run-test` setup (`simple.lisp:38`) — add the
    /// prompt feature and arm the keypress/choose prompt cycling.
    pub fn tracker_setup(&mut self, slots: &[(String, Sexp)]) {
        self.tracker_task = true;
        self.add_visicon(slots);
        // `add-visicon-features` queues the feature (`visicon-updates`); the
        // name is the queued `:add` entry.
        if let Some(VisPending::Add(f)) = self.visicon_pending.last() {
            self.tracker_feature = Some(f.name.clone());
        }
    }

    /// tracker task: set the visicon feature's `prompt` and re-display.
    pub(super) fn tracker_set_prompt(&mut self, prompt: &str) {
        if let Some(name) = self.tracker_feature.clone() {
            if let Some(f) = self.features.iter_mut().find(|f| f.name == name) {
                match f.extra.iter_mut().find(|(k, _)| k == "PROMPT") {
                    Some((_, v)) => *v = Value::Sym(prompt.to_string()),
                    None => f
                        .extra
                        .push(("PROMPT".to_string(), Value::Sym(prompt.to_string()))),
                }
            }
        }
        self.now_display();
    }

    /// extended-motor-actions `maybe-execute-movement`
    /// (`motor-extension.lisp:1069`): run queued movements whose hand's
    /// execution stage is free.
    pub(super) fn zz_maybe_execute(&mut self) {
        loop {
            let mut ran = false;
            let mut i = 0;
            while i < self.man_exec_queue.len() {
                let p = self.man_exec_queue[i].clone();
                let free = if self.cfg.zz_two_exec && !p.both {
                    !self.man_exec[p.hand]
                } else {
                    !self.man_exec[0] && !self.man_exec[1]
                };
                if free {
                    self.man_exec_queue.remove(i);
                    if self.cfg.zz_two_exec && !p.both {
                        self.man_exec[p.hand] = true;
                    } else {
                        self.man_exec[0] = true;
                        self.man_exec[1] = true;
                    }
                    let hand_name = if p.both {
                        "BOTH"
                    } else if p.hand == 0 {
                        "LEFT"
                    } else {
                        "RIGHT"
                    };
                    self.zz_last_style =
                        Some((p.style.clone(), p.style.clone(), hand_name.to_string()));
                    let t = self.time_ms;
                    self.push(
                        t + self.cfg.init_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::InitiationComplete {
                            req_ms: p.req_ms,
                            hand: p.hand,
                            both: p.both,
                            style: p.style.clone(),
                        },
                    );
                    self.push(
                        t + p.finish_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::FinishMovement {
                            req_ms: p.req_ms,
                            hand: p.hand,
                            both: p.both,
                            style: p.style.clone(),
                        },
                    );
                    match p.kind.clone() {
                        ZzKind::Press => {
                            self.push(
                                t + p.exec_ms,
                                Prio::Num(0),
                                Out::Never,
                                Event::MotorNotify {
                                    key: p.key.clone(),
                                    req_ms: p.req_ms,
                                },
                            );
                        }
                        ZzKind::Key {
                            finger,
                            exec2_ms,
                            output,
                            release_exec,
                        } => {
                            if output {
                                self.push(
                                    t + p.exec_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: p.key.clone(),
                                        down: true,
                                        hand: p.hand,
                                        finger,
                                    },
                                );
                            } else if release_exec {
                                // `release`/`release-recoil` put the finger up
                                // at exec (`motor-extension.lisp:1968`/`:2012`).
                                self.push(
                                    t + p.exec_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: p.key.clone(),
                                        down: false,
                                        hand: p.hand,
                                        finger,
                                    },
                                );
                            } else {
                                // `point-finger`'s `set-hand-position`
                                // (`motor-extension.lisp:2053`) has no details
                                // but still wakes conflict resolution.
                                self.push(t + p.exec_ms, Prio::Num(0), Out::Never, Event::Dummy);
                            }
                            if let Some(e2) = exec2_ms {
                                self.push(
                                    t + e2,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: p.key.clone(),
                                        down: false,
                                        hand: p.hand,
                                        finger,
                                    },
                                );
                            }
                            // `perform-movement` marks the finger busy
                            // (`motor-extension.lisp:1186`); the finish handler
                            // frees it before the next movement starts (`:1205`).
                            self.zz_hands[p.hand].busy[finger] = true;
                        }
                        ZzKind::Hand { to_mouse, r, theta } => {
                            self.push(
                                t + p.exec_ms,
                                Prio::Num(0),
                                Out::Low,
                                Event::Emit {
                                    module: "MOTOR".to_string(),
                                    line: format!(
                                        "MOVE-A-HAND RIGHT {} {}",
                                        lisp_s_f32(r),
                                        lisp_s_f32(theta)
                                    ),
                                },
                            );
                            // `perform-movement` auto-releases any finger that
                            // is down `key-release-time` after initiation.
                            let held: Vec<(usize, String)> = (0..5)
                                .filter_map(|f| {
                                    self.zz_hands[p.hand].held[f].clone().map(|k| (f, k))
                                })
                                .collect();
                            for (f, k) in held {
                                self.push(
                                    t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: k,
                                        down: false,
                                        hand: p.hand,
                                        finger: f,
                                    },
                                );
                                self.zz_hands[p.hand].held[f] = None;
                            }
                            if self.zz_mouse_down {
                                self.zz_mouse_down = false;
                                self.push(
                                    t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzMouseRelease { req_ms: p.req_ms },
                                );
                            }
                            let target = if to_mouse {
                                self.zz_mouse_cursor
                            } else {
                                self.zz_home_position(p.hand, 0)
                            };
                            let off = self.zz_hands[p.hand].offsets[0];
                            self.zz_hands[p.hand].loc = (target.0 - off.0, target.1 - off.1);
                        }
                        ZzKind::HandPly { target, r, theta } => {
                            // `queue-output-events` (`motor.lisp:1078`): the
                            // details are `move-a-hand <hand> <r> <theta>`.
                            self.push(
                                t + p.exec_ms,
                                Prio::Num(0),
                                Out::Low,
                                Event::Emit {
                                    module: "MOTOR".to_string(),
                                    line: format!(
                                        "MOVE-A-HAND {} {} {}",
                                        if p.hand == 0 { "LEFT" } else { "RIGHT" },
                                        r,
                                        theta
                                    ),
                                },
                            );
                            // `perform-movement` (`motor-extension.lisp:1163`):
                            // a `hand-ply` releases any held finger
                            // `key-release-time` after initiation.
                            let held: Vec<(usize, String)> = (0..5)
                                .filter_map(|f| {
                                    self.zz_hands[p.hand].held[f].clone().map(|k| (f, k))
                                })
                                .collect();
                            for (f, k) in held {
                                self.push(
                                    t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: k,
                                        down: false,
                                        hand: p.hand,
                                        finger: f,
                                    },
                                );
                                self.zz_hands[p.hand].held[f] = None;
                            }
                            self.zz_hands[p.hand].loc = target;
                        }
                        ZzKind::AllHome { hands } => {
                            // `queue-output-events` (`motor-extension.lisp:2363`):
                            // a `ALL-FINGERS-TO-HOME <hand>` details line per
                            // hand at exec, and the hand reset to home.
                            for h in &hands {
                                self.push(
                                    t + p.exec_ms,
                                    Prio::Num(0),
                                    Out::Low,
                                    Event::Emit {
                                        module: "MOTOR".to_string(),
                                        line: format!(
                                            "ALL-FINGERS-TO-HOME {}",
                                            if *h == 0 { "LEFT" } else { "RIGHT" }
                                        ),
                                    },
                                );
                            }
                            for h in &hands {
                                // Auto-release any held/down finger, then reset
                                // the hand to the home row (`prepare-features`,
                                // `motor-extension.lisp:2271`).
                                let held: Vec<(usize, String)> = (0..5)
                                    .filter_map(|f| {
                                        self.zz_hands[*h].held[f].clone().map(|k| (f, k))
                                    })
                                    .collect();
                                for (f, k) in held {
                                    self.push(
                                        t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                        Prio::Num(0),
                                        Out::Always,
                                        Event::ZzKeyOutput {
                                            key: k,
                                            down: false,
                                            hand: *h,
                                            finger: f,
                                        },
                                    );
                                    self.zz_hands[*h].held[f] = None;
                                }
                                if self.zz_mouse_down {
                                    self.zz_mouse_down = false;
                                    self.push(
                                        t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                        Prio::Num(0),
                                        Out::Always,
                                        Event::ZzMouseRelease { req_ms: p.req_ms },
                                    );
                                }
                                self.zz_hands[*h].loc = if *h == 0 { (4, 4) } else { (7, 4) };
                                self.zz_hands[*h].offsets = if *h == 0 {
                                    LEFT_FINGER_OFFSETS
                                } else {
                                    RIGHT_FINGER_OFFSETS
                                };
                            }
                        }
                        ZzKind::ReleaseAll { hands } => {
                            // `queue-output-events` (`motor-extension.lisp:2610`):
                            // a `RELEASING-ALL-FINGERS` details line at exec; the
                            // finger positions are unchanged.
                            self.push(
                                t + p.exec_ms,
                                Prio::Num(0),
                                Out::Medium,
                                Event::Emit {
                                    module: "MOTOR".to_string(),
                                    line: "RELEASING-ALL-FINGERS".to_string(),
                                },
                            );
                            for h in &hands {
                                // Auto-release any held finger at
                                // `init + key-release-time`.
                                let held: Vec<(usize, String)> = (0..5)
                                    .filter_map(|f| {
                                        self.zz_hands[*h].held[f].clone().map(|k| (f, k))
                                    })
                                    .collect();
                                for (f, k) in held {
                                    self.push(
                                        t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                        Prio::Num(0),
                                        Out::Always,
                                        Event::ZzKeyOutput {
                                            key: k,
                                            down: false,
                                            hand: *h,
                                            finger: f,
                                        },
                                    );
                                    self.zz_hands[*h].held[f] = None;
                                }
                            }
                        }
                    }
                    ran = true;
                    break;
                }
                i += 1;
            }
            if !ran {
                break;
            }
        }
    }

    /// `movement-strike-time` (`motor-extension.lisp:1557`): the delay from
    /// initiation to the finger contacting the key, when
    /// `peck-strike-distance` (`*psd*`) is set.
    pub(super) fn zz_movement_strike_time(&self, tt_ms: i64, r: f32) -> i64 {
        match self.cfg.zz_psd {
            None => tt_ms,
            Some(psd) if psd == 0.0 => tt_ms + self.cfg.zz_kct_ms,
            Some(psd) => {
                let tt = tt_ms as f32 / 1000.0;
                let target = r - psd;
                let mut width = 0.25 * tt;
                let mut guess = 0.75 * tt;
                let mut value = minjerk_dist(guess, r, tt);
                let mut dir: f32 = if value < target { 1.0 } else { -1.0 };
                while (target - value).abs() >= 0.01 {
                    width /= 2.0;
                    guess += dir * width;
                    value = minjerk_dist(guess, r, tt);
                    dir = if value < target { 1.0 } else { -1.0 };
                }
                ((self.cfg.zz_kct_ms as f32 / 1000.0 + guess) * 1000.0).round() as i64
            }
        }
    }

    /// extended-motor-actions mouse request (`move-cursor`/`click-mouse`/
    /// `hold-mouse`/`release-mouse`).
    pub(super) fn zz_mouse_request(
        &mut self,
        cmd: &str,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let loc = slots
            .iter()
            .find(|(s, _)| s == "LOC")
            .and_then(|(_, v)| self.chunk_xy_name(v));
        // `move-cursor` aborts when the cursor is already at the requested
        // target (`motor.lisp:1316`, "r=0 is a no-op"): the trace line is
        // still emitted but no ply runs, so the manual module stays free.
        if cmd == "MOVE-CURSOR" {
            if let Some((tx, ty, name)) = &loc {
                if *tx == self.zz_mouse_pos.0 && *ty == self.zz_mouse_pos.1 {
                    self.now(
                        Prio::Num(0),
                        Out::Low,
                        Event::Emit {
                            module: "MOTOR".to_string(),
                            line: format!("MOVE-CURSOR LOC {name}"),
                        },
                    );
                    return Ok(());
                }
            }
        }
        // `move-cursor`'s Fitts time uses the target's *approach width*
        // (`motor.lisp:1318`): the width/height the target presents to the
        // approach angle, converted to degrees of visual angle
        // (`approach-width`, `vision.lisp:3717`; `pm-pixels-to-angle`,
        // `device-interface.lisp:743`). A target with no width/height falls
        // back to the vision module's `default-width` (the
        // `:default-target-width` parameter, 1.0).
        let fit_ms = if cmd == "MOVE-CURSOR" {
            loc.as_ref().map(|(tx, ty, name)| {
                let (cx, cy) = (self.zz_mouse_pos.0, self.zz_mouse_pos.1);
                let r_pix = (((tx - cx) as f32).powi(2) + ((ty - cy) as f32).powi(2)).sqrt();
                let theta = ((ty - cy) as f32).atan2((tx - cx) as f32);
                let w_dva = self
                    .resolve_loc(name)
                    .ok()
                    .map(|i| (self.features[i].width, self.features[i].height))
                    .filter(|&(w, h)| w > 0 && h > 0)
                    .map(|(w, h)| pm_pixels_to_angle(approach_width(w as f32, h as f32, theta)))
                    .unwrap_or(1.0);
                let mut coef = self.cfg.touch_cursor_coeff;
                if let Some(d) = self.cfg.zz_cdd {
                    if self.zz_mouse_down {
                        coef *= 2.0f32.powf(d as f32);
                    }
                }
                let r_dva = pm_pixels_to_angle(r_pix);
                let fit = (coef * (r_dva / w_dva + 0.5).log2())
                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                (fit * 1000.0).round() as i64
            })
        } else {
            None
        };
        // `(class, style-name, fresh-features)`: the details use the class
        // (`type-of`), `compute-prep-time` compares the style-name.
        let (class, style_name, fresh) = match cmd {
            "MOVE-CURSOR" => ("CURSOR-PLY", "CURSOR-PLY", 4i64),
            "CLICK-MOUSE" => ("PUNCH", "PUNCH", 3),
            "HOLD-MOUSE" => ("HOLD-PUNCH", "PUNCH", 3),
            "RELEASE-MOUSE" => ("RELEASE", "PUNCH", 3),
            _ => unreachable!(),
        };
        let feats = match &self.zz_last_style {
            Some((_, ls, _)) if ls == style_name => 0,
            _ => fresh,
        };
        self.zz_last_style = Some((
            class.to_string(),
            style_name.to_string(),
            "RIGHT".to_string(),
        ));
        self.man_prep_busy = true;
        self.man_last_prep = Some(1);
        self.zz_last_cmd[1] = class.to_string();
        if self.cfg.zz_two_proc {
            self.man_proc[1] = true;
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        let prep = feats * self.cfg.feat_prep_ms;
        // `hold-mouse`/`release-mouse` call `hold-punch`/`release` directly
        // (`motor-extension.lisp:2469`/`2486`), so they have no request trace.
        let trace = match cmd {
            "MOVE-CURSOR" => Some(format!(
                "MOVE-CURSOR LOC {}",
                loc.as_ref().map(|l| l.2.clone()).unwrap_or_default()
            )),
            "CLICK-MOUSE" => Some("CLICK-MOUSE".to_string()),
            _ => None,
        };
        if let Some(line) = trace {
            self.now(
                Prio::Num(0),
                Out::Low,
                Event::Emit {
                    module: "MOTOR".to_string(),
                    line,
                },
            );
        }
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzMousePrep {
                cmd: cmd.to_string(),
                req_ms,
                loc,
                fit_ms,
            },
        );
        Ok(())
    }

    /// extended-motor-actions: a finger's absolute keyboard location.
    pub(super) fn zz_finger_loc(&self, hand: usize, finger: usize) -> (i64, i64) {
        let h = &self.zz_hands[hand];
        (h.loc.0 + h.offsets[finger].0, h.loc.1 + h.offsets[finger].1)
    }

    /// extended-motor-actions: a finger's home keyboard location.
    pub(super) fn zz_home_position(&self, hand: usize, finger: usize) -> (i64, i64) {
        let home = if hand == 0 { (4, 4) } else { (7, 4) };
        let offs = if hand == 0 {
            LEFT_FINGER_OFFSETS
        } else {
            RIGHT_FINGER_OFFSETS
        };
        (home.0 + offs[finger].0, home.1 + offs[finger].1)
    }

    /// `(style-name, num-possible-feats, hfrt)` for a movement class
    /// (`motor-extension.lisp:1861`+ / `motor.lisp:984`).
    pub(super) fn zz_style_info(class: &str) -> (&'static str, i64, bool) {
        match class {
            "PECK" | "PECK-RECOIL" | "HOLD-PECK" | "POINT-FINGER" | "RELEASE-RECOIL" => {
                ("PECK", 5, true)
            }
            "PUNCH" | "HOLD-PUNCH" | "RELEASE" | "DELAYED-PUNCH" => ("PUNCH", 3, false),
            // `hand-ply` (`motor.lisp:1091`): `ply`'s `num-possible-feats`
            // `:around` drops the `:dummy` finger, so 4 features to prepare.
            "HAND-PLY" => ("PLY", 4, true),
            _ => ("PLY", 3, false),
        }
    }

    /// `feat-differences` (`motor.lisp:970`) / `compute-prep-time`
    /// (`general-pm.lisp:670`): how many features need preparing given the
    /// previously prepared movement.
    pub(super) fn zz_feat_differences(
        &self,
        style_name: &str,
        maxfeats: i64,
        hand: usize,
        finger: usize,
        r: f32,
        theta: f32,
    ) -> i64 {
        match &self.zz_last_move {
            None => maxfeats,
            Some(prev) if prev.style_name != style_name => maxfeats,
            Some(prev) if prev.hfrt => {
                let maxfeats = prev.maxfeats;
                if prev.hand != hand {
                    maxfeats - 1
                } else if prev.finger != finger {
                    maxfeats - 2
                } else {
                    let mut n = 0;
                    if (prev.r - r).abs() >= 2.0 {
                        n += 1;
                    }
                    if !direction_eq(prev.theta, theta) {
                        n += 1;
                    }
                    n
                }
            }
            Some(prev) => {
                if prev.hand != hand {
                    2
                } else if prev.finger != finger {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// `extended-finger-keyboard` (`motor-extension.lisp:2103`): resolve a key
    /// to a hand/finger, then dispatch "same-loc" or "move-loc".
    pub(super) fn zz_extended_finger_keyboard(
        &mut self,
        cmd: &str,
        key: &str,
        same_style: &str,
        move_style: &str,
    ) -> Result<(), EngineError> {
        let Some((hand, finger)) = key_hand_finger(key) else {
            return Ok(());
        };
        let Some(key_loc) = key_xy(key) else {
            return Ok(());
        };
        let finger_loc = self.zz_finger_loc(hand, finger);
        if key_loc == finger_loc {
            self.zz_key_request(
                cmd,
                same_style,
                key,
                hand,
                finger,
                0.0,
                0.0,
                Some(key_loc),
                0,
            )
        } else {
            let dx = (key_loc.0 - finger_loc.0) as f64;
            let dy = (key_loc.1 - finger_loc.1) as f64;
            let r = (dx * dx + dy * dy).sqrt() as f32;
            let theta = dy.atan2(dx) as f32;
            self.zz_key_request(
                cmd,
                move_style,
                key,
                hand,
                finger,
                r,
                theta,
                Some(key_loc),
                0,
            )
        }
    }

    /// Schedule a keyboard movement style's prep/exec/finish.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn zz_key_request(
        &mut self,
        cmd: &str,
        class: &str,
        key: &str,
        hand: usize,
        finger: usize,
        r: f32,
        theta: f32,
        loc: Option<(i64, i64)>,
        delay_ms: i64,
    ) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let (style_name, maxfeats, hfrt) = Self::zz_style_info(class);
        // `compute-prep-time`/`feat-differences` (`general-pm.lisp:670`,
        // `motor.lisp:970`).
        let feats = self.zz_feat_differences(style_name, maxfeats, hand, finger, r, theta);
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        self.zz_last_style = Some((
            class.to_string(),
            style_name.to_string(),
            hand_name.to_string(),
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: style_name.to_string(),
            hand,
            finger,
            r,
            theta,
            maxfeats,
            hfrt,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(hand);
        self.zz_last_cmd[hand] = class.to_string();
        if self.cfg.zz_two_proc {
            self.man_proc[hand] = true;
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        let prep = feats * self.cfg.feat_prep_ms;
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzKeyPrep {
                cmd: cmd.to_string(),
                style: class.to_string(),
                key: key.to_string(),
                hand,
                finger,
                r,
                theta,
                loc,
                delay_ms,
                req_ms,
            },
        );
        Ok(())
    }

    /// extended-motor-actions keyboard/mouse-high-level commands.
    pub(super) fn zz_key_cmd(
        &mut self,
        cmd: &str,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let key = slots
            .iter()
            .find(|(s, _)| s == "KEY")
            .map(|(_, v)| name_of(v))
            .unwrap_or_default();
        match cmd {
            "HOLD-KEY" => self.zz_extended_finger_keyboard(cmd, &key, "HOLD-PUNCH", "HOLD-PECK"),
            "HIT-KEY" => self.zz_extended_finger_keyboard(cmd, &key, "PUNCH", "PECK"),
            "TYPE-KEY" => self.zz_extended_finger_keyboard(cmd, &key, "PUNCH", "PECK-RECOIL"),
            "MOVE-TO-KEY" => {
                self.zz_extended_finger_keyboard(cmd, &key, "POINT-FINGER", "POINT-FINGER")
            }
            "RELEASE-KEY" => self.zz_extended_finger_keyboard(cmd, &key, "RELEASE", "RELEASE"),
            "RELEASE-KEY-TO-HOME" => self.zz_release_key_to_home(&key),
            "MOVE-FINGER-TO-HOME" => self.zz_move_finger_to_home(slots),
            "ALL-FINGERS-TO-HOME" => self.zz_all_fingers_to_home(slots),
            "HAND-TO-MOUSE" => self.zz_hand_to(true),
            "HAND-TO-HOME" => self.zz_hand_to(false),
            _ => Ok(()),
        }
    }

    /// A direct style request (`handle-style-request`, `motor.lisp:1447`):
    /// trace the request, then schedule the style with the spec's features.
    pub(super) fn zz_direct_style(
        &mut self,
        cmd: &str,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let hand = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
            .unwrap_or(0);
        let finger = slots
            .iter()
            .find(|(s, _)| s == "FINGER")
            .and_then(|(_, v)| Self::touch_finger_idx(&name_of(v)))
            .unwrap_or(0);
        let r_val = slots.iter().find(|(s, _)| s == "R").map(|(_, v)| v);
        let theta_val = slots.iter().find(|(s, _)| s == "THETA").map(|(_, v)| v);
        let r = r_val.and_then(num_of).unwrap_or(0.0) as f32;
        let theta = theta_val.and_then(num_of).unwrap_or(0.0) as f32;
        let r_text = r_val.map(lisp_num_str).unwrap_or_else(|| "0".to_string());
        let theta_text = theta_val
            .map(lisp_num_str)
            .unwrap_or_else(|| "0".to_string());
        self.zz_direct_style_trace(cmd, hand, finger, &r_text, &theta_text);
        // `move-a-finger` shifts the offset by the polar vector.
        let cur = self.zz_finger_loc(hand, finger);
        let target = if r == 0.0 {
            cur
        } else {
            (
                (cur.0 as f32 + r * theta.cos()).round() as i64,
                (cur.1 as f32 + r * theta.sin()).round() as i64,
            )
        };
        // A direct style has no `key` slot; the device names the output from
        // the finger's location (`motor.lisp:960`).
        let key = xy_key(target.0, target.1).unwrap_or("").to_string();
        if !self.cfg.zz_ext {
            self.man_exec_busy = true;
        }
        self.zz_key_request(cmd, cmd, &key, hand, finger, r, theta, Some(target), 0)
    }

    /// `handle-style-request` details (`motor.lisp:1459`): the style name
    /// followed by its feature slots.
    pub(super) fn zz_direct_style_trace(
        &mut self,
        class: &str,
        hand: usize,
        finger: usize,
        r_text: &str,
        theta_text: &str,
    ) {
        let h = if hand == 0 { "LEFT" } else { "RIGHT" };
        let f = ["INDEX", "MIDDLE", "RING", "PINKIE", "THUMB"][finger];
        let (_, _, hfrt) = Self::zz_style_info(class);
        let line = if hfrt {
            format!("{class} HAND {h} FINGER {f} R {r_text} THETA {theta_text}")
        } else {
            format!("{class} HAND {h} FINGER {f}")
        };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line,
            },
        );
    }

    /// `all-fingers-to-home` (`motor-extension.lisp:2401`): move the requested
    /// hand (or both) if it is not already at the home row.
    pub(super) fn zz_all_fingers_to_home(
        &mut self,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        // `:2405`: no `hand` slot means both hands; otherwise the named hand
        // (`both` is the two-hand case).
        let hand_slot = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .map(|(_, v)| name_of(v).to_uppercase());
        let candidates: Vec<usize> = match hand_slot.as_deref() {
            None | Some("BOTH") => vec![0, 1],
            Some("LEFT") => vec![0],
            Some("RIGHT") => vec![1],
            Some(_) => return Ok(()),
        };
        let mut hands: Vec<usize> = Vec::new();
        for h in candidates {
            if self.zz_hand_needs_home(h) {
                hands.push(h);
            }
        }
        if hands.is_empty() {
            return Ok(());
        }
        let both = hands.len() == 2;
        let single = if both { 1 } else { hands[0] };
        // `r` = max hand-ply distance to the home row (`:2317`/`:2346`).
        let r = hands
            .iter()
            .map(|h| {
                let home = if *h == 0 { (4, 4) } else { (7, 4) };
                let cur = self.zz_hands[*h].loc;
                let dx = (home.0 - cur.0) as f64;
                let dy = (home.1 - cur.1) as f64;
                (dx * dx + dy * dy).sqrt() as f32
            })
            .fold(0.0f32, f32::max);
        // `compute-prep-time`: the previous style-name differs (`peck` vs
        // `all-home`), so `num-to-prepare` = hand + r minus the direction
        // (`:2265`) = 2.
        let feats = 2i64;
        self.zz_last_style = Some((
            "ALL-FINGERS-TO-HOME".to_string(),
            "ALL-HOME".to_string(),
            if both {
                "BOTH".to_string()
            } else if single == 0 {
                "LEFT".to_string()
            } else {
                "RIGHT".to_string()
            },
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: "ALL-HOME".to_string(),
            hand: single,
            finger: 4,
            r,
            theta: 0.0,
            maxfeats: 5,
            hfrt: true,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(single);
        for h in &hands {
            self.zz_last_cmd[*h] = "ALL-FINGERS-TO-HOME".to_string();
        }
        if self.cfg.zz_two_proc {
            for h in &hands {
                self.man_proc[*h] = true;
            }
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        let prep = feats * self.cfg.feat_prep_ms;
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzAllHomePrep { req_ms, hands, r },
        );
        Ok(())
    }

    /// `release-all-fingers` (`motor-extension.lisp:2616`): release the held
    /// fingers of whichever hands have one down; with none, warn and do
    /// nothing.
    pub(super) fn zz_release_all_fingers(&mut self) -> Result<(), EngineError> {
        let mut hands: Vec<usize> = Vec::new();
        for h in 0..2 {
            if (0..5).any(|f| self.zz_hands[h].down[f]) {
                hands.push(h);
            }
        }
        if hands.is_empty() {
            // `:2637`: `print-warning`, no movement.
            self.emit_warning(
                "Release-all-fingers action called when no fingers need to be released.",
            );
            return Ok(());
        }
        let req_ms = self.time_ms;
        let both = hands.len() == 2;
        let single = if both { 1 } else { hands[0] };
        let hand_name = if both {
            "BOTH"
        } else if single == 0 {
            "LEFT"
        } else {
            "RIGHT"
        };
        // The request trace is an explicit `:details` string, so it stays
        // lowercase (`motor-extension.lisp:2628`).
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: "release-all-fingers".to_string(),
            },
        );
        self.zz_last_style = Some((
            "RELEASE-ALL-FINGERS".to_string(),
            "RELEASE-ALL-FINGERS".to_string(),
            hand_name.to_string(),
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: "RELEASE-ALL-FINGERS".to_string(),
            hand: single,
            finger: ZZ_DUMMY_FINGER,
            r: 0.0,
            theta: 0.0,
            maxfeats: 1,
            hfrt: false,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(single);
        for h in &hands {
            self.zz_last_cmd[*h] = "RELEASE-ALL-FINGERS".to_string();
        }
        if self.cfg.zz_two_proc {
            for h in &hands {
                self.man_proc[*h] = true;
            }
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        // `num-to-prepare` is 1 (`release-all-fingers` feature-slots is
        // `(hand)` and its `:around` drops one, `:2524`).
        let prep = self.cfg.feat_prep_ms;
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzReleaseAllPrep { req_ms, hands },
        );
        Ok(())
    }

    /// `all-fingers-to-home` (`motor-extension.lisp:2419`): whether a hand
    /// needs moving — off the home location, any finger down, or a finger off
    /// its home position.
    pub(super) fn zz_hand_needs_home(&self, h: usize) -> bool {
        let home_loc = if h == 0 { (4, 4) } else { (7, 4) };
        if self.zz_hands[h].loc != home_loc {
            return true;
        }
        for f in 0..5 {
            if self.zz_hands[h].down[f] {
                return true;
            }
            if self.zz_finger_loc(h, f) != self.zz_home_position(h, f) {
                return true;
            }
        }
        false
    }

    /// `move-finger-to-home` (`motor-extension.lisp:2222`): if the finger is
    /// not already home, `point-finger` it back.
    pub(super) fn zz_move_finger_to_home(
        &mut self,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let Some(hand) = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
        else {
            return Ok(());
        };
        let Some(finger) = slots
            .iter()
            .find(|(s, _)| s == "FINGER")
            .and_then(|(_, v)| Self::touch_finger_idx(&name_of(v)))
        else {
            return Ok(());
        };
        let home = self.zz_home_position(hand, finger);
        let cur = self.zz_finger_loc(hand, finger);
        if home == cur {
            // `:2236`: warning only, no movement.
            return Ok(());
        }
        let dx = (home.0 - cur.0) as f64;
        let dy = (home.1 - cur.1) as f64;
        let r = (dx * dx + dy * dy).sqrt() as f32;
        let theta = dy.atan2(dx) as f32;
        self.zz_key_request(
            "MOVE-FINGER-TO-HOME",
            "POINT-FINGER",
            "",
            hand,
            finger,
            r,
            theta,
            Some(home),
            0,
        )
    }

    /// `release-key-to-home` (`motor-extension.lisp:2183`): release the key
    /// under the finger, recoiling to home if the finger is off home.
    pub(super) fn zz_release_key_to_home(&mut self, key: &str) -> Result<(), EngineError> {
        let Some((hand, finger)) = key_hand_finger(key) else {
            return Ok(());
        };
        let Some(key_loc) = key_xy(key) else {
            return Ok(());
        };
        let finger_loc = self.zz_finger_loc(hand, finger);
        if key_loc != finger_loc {
            // `:2195`: warning only, no action.
            return Ok(());
        }
        let home = self.zz_home_position(hand, finger);
        if home == finger_loc {
            self.zz_key_request(
                "RELEASE-KEY-TO-HOME",
                "RELEASE",
                key,
                hand,
                finger,
                0.0,
                0.0,
                None,
                0,
            )
        } else {
            let dx = (home.0 - finger_loc.0) as f64;
            let dy = (home.1 - finger_loc.1) as f64;
            let r = (dx * dx + dy * dy).sqrt() as f32;
            let theta = dy.atan2(dx) as f32;
            self.zz_key_request(
                "RELEASE-KEY-TO-HOME",
                "RELEASE-RECOIL",
                key,
                hand,
                finger,
                r,
                theta,
                Some(home),
                0,
            )
        }
    }

    /// `isa clear`: clear the manual module and free its state.
    fn manual_clear(&mut self) -> Result<(), EngineError> {
        self.man_prep_busy = true;
        self.man_exec_busy = true;
        self.zz_last_move = None;
        self.zz_last_style = None;
        self.now(
            Prio::Num(0),
            Out::Always,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: "CLEAR".to_string(),
            },
        );
        self.push(
            self.time_ms + self.cfg.feat_prep_ms,
            Prio::Num(0),
            Out::Always,
            Event::ChangeState {
                module: "MANUAL".to_string(),
            },
        );
        Ok(())
    }

    /// `hand-to-mouse`/`hand-to-home` (a `hand-ply`): move the right hand to
    /// the mouse / home, auto-releasing any held key.
    pub(super) fn zz_hand_to(&mut self, to_mouse: bool) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let target = if to_mouse {
            self.zz_mouse_cursor
        } else {
            self.zz_home_position(1, 0)
        };
        let (cx, cy) = self.zz_finger_loc(1, 0);
        let dx = (target.0 - cx) as f64;
        let dy = (target.1 - cy) as f64;
        let r = (dx * dx + dy * dy).sqrt() as f32;
        let theta = dy.atan2(dx) as f32;
        let (style_name, maxfeats, hfrt, feats) = if self.cfg.zz_ext {
            ("PLY", 3, false, 4)
        } else {
            let (style_name, maxfeats, hfrt) = Self::zz_style_info("HAND-PLY");
            let feats =
                self.zz_feat_differences(style_name, maxfeats, 1, ZZ_DUMMY_FINGER, r, theta);
            (style_name, maxfeats, hfrt, feats)
        };
        self.zz_last_style = Some((
            "HAND-PLY".to_string(),
            "HAND-PLY".to_string(),
            "RIGHT".to_string(),
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: style_name.to_string(),
            hand: 1,
            finger: ZZ_DUMMY_FINGER,
            r,
            theta,
            maxfeats,
            hfrt,
        });
        self.man_prep_busy = true;
        if !self.cfg.zz_ext {
            self.man_exec_busy = true;
        }
        self.man_last_prep = Some(1);
        self.zz_last_cmd[1] = "HAND-PLY".to_string();
        if self.cfg.zz_two_proc {
            self.man_proc[1] = true;
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        if !to_mouse {
            self.zz_hands[1].offsets = RIGHT_FINGER_OFFSETS;
        }
        // `HAND-TO-MOUSE`/`HAND-TO-HOME` request trace (custom).
        let line = if to_mouse {
            "HAND-TO-MOUSE".to_string()
        } else {
            "HAND-TO-HOME".to_string()
        };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line,
            },
        );
        self.push(
            self.time_ms + feats * self.cfg.feat_prep_ms,
            Prio::Num(0),
            Out::Always,
            Event::ZzHandPrep {
                req_ms,
                r,
                theta,
                to_mouse,
            },
        );
        Ok(())
    }

    /// `hand-to-keypad` (`keyboard.lisp:617`): move the right hand to the
    /// keypad's home position and install its finger offsets.
    pub(super) fn zz_hand_to_keypad(&mut self) -> Result<(), EngineError> {
        let hand = 1;
        let target = (19, 4);
        let (hx, hy) = self.zz_hands[hand].loc;
        let dx = (target.0 - hx) as f64;
        let dy = (target.1 - hy) as f64;
        let r = (dx * dx + dy * dy).sqrt() as f32;
        let theta = dy.atan2(dx) as f32;
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: "HAND-TO-KEYPAD".to_string(),
            },
        );
        self.zz_hands[hand].offsets = KEYPAD_FINGER_OFFSETS;
        if !self.cfg.zz_ext {
            self.man_exec_busy = true;
        }
        self.zz_hand_ply_request(
            "HAND-PLY",
            hand,
            r,
            theta,
            lisp_s_f32(r),
            lisp_s_f32(theta),
            target,
            4.0,
            false,
        )
    }

    /// Schedule a `hand-ply` (a `ply` that moves the hand loc) movement.
    /// Used by `prepare`/`execute` and `point-hand-at-key`
    /// (`motor.lisp:1096`/`:1149`). `r_str`/`theta_str` are the pre-rendered
    /// `~a` spellings for the `MOVE-A-HAND` details; `deferred` marks a
    /// `prepare` whose execution waits for an `execute` request.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn zz_hand_ply_request(
        &mut self,
        class: &str,
        hand: usize,
        r: f32,
        theta: f32,
        r_str: String,
        theta_str: String,
        target: (i64, i64),
        twidth: f32,
        deferred: bool,
    ) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let (style_name, maxfeats, hfrt) = Self::zz_style_info(class);
        let feats = self.zz_feat_differences(style_name, maxfeats, hand, ZZ_DUMMY_FINGER, r, theta);
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        self.zz_last_style = Some((
            class.to_string(),
            style_name.to_string(),
            hand_name.to_string(),
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: style_name.to_string(),
            hand,
            finger: ZZ_DUMMY_FINGER,
            r,
            theta,
            maxfeats,
            hfrt,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(hand);
        self.zz_last_cmd[hand] = class.to_string();
        if self.cfg.zz_two_proc {
            self.man_proc[hand] = true;
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        let prep = feats * self.cfg.feat_prep_ms;
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzHandPlyPrep {
                class: class.to_string(),
                hand,
                r,
                theta,
                r_str,
                theta_str,
                target,
                twidth,
                req_ms,
                deferred,
            },
        );
        Ok(())
    }

    /// `prepare` (`general-pm.lisp:562`): build a `hand-ply` and prepare it,
    /// but do not execute it until an `execute` request arrives. The trace is
    /// the event name plus its params (`scheduling.lisp` `format-event`).
    pub(super) fn zz_prepare(&mut self, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let hand = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
            .unwrap_or(1);
        let r_val = slots.iter().find(|(s, _)| s == "R").map(|(_, v)| v);
        let theta_val = slots.iter().find(|(s, _)| s == "THETA").map(|(_, v)| v);
        let r = r_val.and_then(num_of).unwrap_or(0.0) as f32;
        let theta = theta_val.and_then(num_of).unwrap_or(0.0) as f32;
        let r_str = r_val.map(lisp_num_str).unwrap_or_else(|| "0".to_string());
        let theta_str = theta_val
            .map(lisp_num_str)
            .unwrap_or_else(|| "0".to_string());
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: format!("PREPARE HAND-PLY HAND {hand_name} R {r_str} THETA {theta_str}"),
            },
        );
        // `prepare-features` (`motor.lisp:1106`): `move-a-hand` shifts the
        // hand loc by the polar vector (`polar-move-xy` rounds each axis).
        let (hx, hy) = self.zz_hands[hand].loc;
        let target = (
            (hx as f32 + r * theta.cos()).round() as i64,
            (hy as f32 + r * theta.sin()).round() as i64,
        );
        self.zz_hand_ply_request(
            "HAND-PLY", hand, r, theta, r_str, theta_str, target, 1.0, true,
        )
    }

    /// `execute` (`general-pm.lisp:580`): queue the last prepared movement and
    /// run it.
    pub(super) fn zz_execute(&mut self) -> Result<(), EngineError> {
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: "EXECUTE".to_string(),
            },
        );
        if let Some(p) = self.zz_prepared.take() {
            self.man_exec_queue.push(p);
            self.zz_maybe_execute();
        }
        Ok(())
    }

    /// `point-hand-at-key` (`keyboard.lisp:653`): point the hand's index finger
    /// at `to-key` via a `hand-ply`, leaving the finger offsets unchanged.
    pub(super) fn zz_point_hand_at_key(
        &mut self,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let hand = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
            .unwrap_or(1);
        let to_key = slots
            .iter()
            .find(|(s, _)| s == "TO-KEY")
            .map(|(_, v)| name_of(v))
            .unwrap_or_default();
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        let mut line = format!("POINT-HAND-AT-KEY HAND {hand_name} TO-KEY {to_key}");
        if let Some((_, offsets)) = slots.iter().find(|(s, _)| s == "OFFSETS") {
            line.push_str(&format!(" OFFSETS {}", offsets.show()));
        }
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line,
            },
        );
        if let Some((_, offsets)) = slots.iter().find(|(s, _)| s == "OFFSETS") {
            match offsets {
                Value::Sym(s) if s.eq_ignore_ascii_case("STANDARD") => {
                    self.zz_hands[hand].offsets = if hand == 0 {
                        LEFT_FINGER_OFFSETS
                    } else {
                        RIGHT_FINGER_OFFSETS
                    };
                }
                Value::List(items) => {
                    for item in items {
                        let Value::List(parts) = item else {
                            continue;
                        };
                        let (Some(Value::Sym(finger)), Some(x), Some(y)) = (
                            parts.first(),
                            parts.get(1).and_then(num_of),
                            parts.get(2).and_then(num_of),
                        ) else {
                            continue;
                        };
                        if let Some(f) = Self::touch_finger_idx(finger) {
                            self.zz_hands[hand].offsets[f] = (x as i64, y as i64);
                        }
                    }
                }
                _ => {}
            }
        }
        let Some(new_loc) = key_xy(&to_key) else {
            return Ok(());
        };
        let (hx, hy) = self.zz_hands[hand].loc;
        let dx = (new_loc.0 - hx) as f64;
        let dy = (new_loc.1 - hy) as f64;
        let r = (dx * dx + dy * dy).sqrt() as f32;
        let theta = dy.atan2(dx) as f32;
        let target = (
            (hx as f32 + r * theta.cos()).round() as i64,
            (hy as f32 + r * theta.sin()).round() as i64,
        );
        if !self.cfg.zz_ext {
            self.man_exec_busy = true;
        }
        self.zz_hand_ply_request(
            "HAND-PLY",
            hand,
            r,
            theta,
            lisp_s_f32(r),
            lisp_s_f32(theta),
            target,
            1.0,
            false,
        )
    }

    /// `delayed-punch` (`motor-extension.lisp:2699`): a punch whose release is
    /// `delay` after the strike (no `key-release-time`). `delay` is
    /// `default`/`fast`/`slow`/a positive number (`parse-delay-time`, `:2685`).
    pub(super) fn zz_delayed_punch(
        &mut self,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let hand = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
            .unwrap_or(1);
        let finger = slots
            .iter()
            .find(|(s, _)| s == "FINGER")
            .and_then(|(_, v)| Self::touch_finger_idx(&name_of(v)))
            .unwrap_or(0);
        // `parse-delay-time` (`:2685`): nil/`default` -> default; a positive
        // number as-is; `slow`/`fast` -> the named delays.
        let delay_ms = match slots.iter().find(|(s, _)| s == "DELAY").map(|(_, v)| v) {
            None => self.cfg.zz_dpd_ms,
            Some(Value::Sym(s)) if s.eq_ignore_ascii_case("DEFAULT") => self.cfg.zz_dpd_ms,
            Some(Value::Sym(s)) if s.eq_ignore_ascii_case("FAST") => self.cfg.zz_fpd_ms,
            Some(Value::Sym(s)) if s.eq_ignore_ascii_case("SLOW") => self.cfg.zz_spd_ms,
            Some(v) => num_of(v)
                .filter(|x| *x > 0.0)
                .map(|x| (x * 1000.0).round() as i64)
                .unwrap_or(self.cfg.zz_dpd_ms),
        };
        let delay_str = lisp_s_f32(delay_ms as f32 / 1000.0);
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        let finger_name = ["INDEX", "MIDDLE", "RING", "PINKIE", "THUMB"][finger];
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: format!(
                    "DELAYED-PUNCH HAND {hand_name} FINGER {finger_name} DELAY {delay_str}"
                ),
            },
        );
        // The device names the key from the finger's location (`motor.lisp:960`);
        // `delayed-punch` doesn't move the finger.
        let floc = self.zz_finger_loc(hand, finger);
        let key = xy_key(floc.0, floc.1).unwrap_or("").to_string();
        self.zz_key_request(
            "DELAYED-PUNCH",
            "DELAYED-PUNCH",
            &key,
            hand,
            finger,
            0.0,
            0.0,
            None,
            delay_ms,
        )
    }

    /// Manual request routing: press-key only.
    pub(super) fn manual_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        let (ctype, slots) = match &spec {
            NewChunk::Spec { ctype, slots, .. } => (ctype.clone(), slots.clone()),
            _ => return Err(unimpl("non-spec manual request")),
        };
        // `isa press-key key X` (no `cmd`).
        let mut cmd: Option<String> = if ctype.eq_ignore_ascii_case("PRESS-KEY") {
            Some("PRESS-KEY".to_string())
        } else if STANDARD_MANUAL_CMDS
            .iter()
            .any(|c| c.eq_ignore_ascii_case(&ctype))
        {
            Some(ctype.to_uppercase())
        } else if self.cfg.touch || self.cfg.zz_ext {
            // act-touch / extended-motor-actions request chunk types carry
            // the command as their type name (`extend-manual-requests-fct`).
            let t = ctype.to_uppercase();
            if matches!(
                t.as_str(),
                "TAP"
                    | "TAP-HOLD"
                    | "TAP-RELEASE"
                    | "SWIPE"
                    | "PINCH"
                    | "ROTATE"
                    | "MOVE-HAND-TOUCH"
                    | "HAND-TO-TOUCH"
                    | "MOVE-CURSOR"
                    | "CLICK-MOUSE"
                    | "HOLD-MOUSE"
                    | "RELEASE-MOUSE"
                    | "HOLD-KEY"
                    | "HIT-KEY"
                    | "TYPE-KEY"
                    | "MOVE-TO-KEY"
                    | "RELEASE-KEY"
                    | "RELEASE-KEY-TO-HOME"
                    | "MOVE-FINGER-TO-HOME"
                    | "ALL-FINGERS-TO-HOME"
                    | "HAND-TO-MOUSE"
                    | "HAND-TO-HOME"
                    | "HOLD-PUNCH"
                    | "HOLD-PECK"
                    | "PUNCH"
                    | "PECK"
                    | "PECK-RECOIL"
                    | "POINT-FINGER"
                    | "RELEASE"
                    | "PREPARE"
                    | "EXECUTE"
                    | "POINT-HAND-AT-KEY"
                    | "DELAYED-PUNCH"
                    | "RELEASE-ALL-FINGERS"
            ) {
                Some(t)
            } else {
                None
            }
        } else {
            None
        };
        let mut key: Option<String> = None;
        for (s, v) in &slots {
            match (s.as_str(), v) {
                ("CMD", Value::Sym(c)) => cmd = Some(c.to_uppercase()),
                ("KEY", Value::Sym(k)) => key = Some(k.clone()),
                ("KEY", Value::Str(k)) => key = Some(k.clone()),
                ("KEY", Value::Int(i)) => key = Some(i.to_string()),
                ("KEY", Value::Num(x)) => key = Some(lisp_s_f32(*x as f32)),
                // A built-in style's own slots (hand/finger/r/theta/loc) are
                // consumed by its handler below.
                (s, _) if STANDARD_MANUAL_SLOTS.contains(&s) => {}
                _ if self.cfg.touch || self.cfg.zz_ext => {}
                got => {
                    return Err(unimpl(format!("manual request slot {got:?}")));
                }
            }
        }
        match (cmd.as_deref(), key) {
            (Some("PRESS-KEY"), Some(k)) => {
                Self::key_spec(&k)?;
                let req_ms = self.time_ms;
                // `cmd press-key` vs `isa press-key` (no cmd).
                let isa = !slots.iter().any(|(s, _)| s == "CMD");
                self.now(
                    Prio::Num(0),
                    Out::Low,
                    Event::PressKey {
                        key: k,
                        req_ms,
                        isa,
                    },
                );
                Ok(())
            }
            (Some(c), None)
                if self.cfg.touch
                    && matches!(
                        c,
                        "TAP"
                            | "TAP-HOLD"
                            | "TAP-RELEASE"
                            | "SWIPE"
                            | "PINCH"
                            | "ROTATE"
                            | "MOVE-HAND-TOUCH"
                            | "HAND-TO-TOUCH"
                    ) =>
            {
                self.touch_request(&slots)
            }
            (Some(c), None)
                if matches!(
                    c,
                    "MOVE-CURSOR" | "CLICK-MOUSE" | "HOLD-MOUSE" | "RELEASE-MOUSE"
                ) =>
            {
                self.zz_mouse_request(c, &slots)
            }
            (Some("HAND-TO-MOUSE"), _) => self.zz_hand_to(true),
            (Some("HAND-TO-HOME"), _) => self.zz_hand_to(false),
            (Some("HAND-TO-KEYPAD"), _) => self.zz_hand_to_keypad(),
            (Some("POINT-HAND-AT-KEY"), _) => self.zz_point_hand_at_key(&slots),
            (Some("CLEAR"), None) => self.manual_clear(),
            (Some(c), _)
                if self.cfg.zz_ext
                    && matches!(
                        c,
                        "HOLD-KEY"
                            | "HIT-KEY"
                            | "TYPE-KEY"
                            | "MOVE-TO-KEY"
                            | "RELEASE-KEY"
                            | "RELEASE-KEY-TO-HOME"
                            | "MOVE-FINGER-TO-HOME"
                            | "ALL-FINGERS-TO-HOME"
                    ) =>
            {
                self.zz_key_cmd(c, &slots)
            }
            (Some(c), _)
                if matches!(
                    c,
                    "HOLD-PUNCH"
                        | "HOLD-PECK"
                        | "PUNCH"
                        | "PECK"
                        | "PECK-RECOIL"
                        | "POINT-FINGER"
                        | "RELEASE"
                ) =>
            {
                self.zz_direct_style(c, &slots)
            }
            (Some("PREPARE"), _) if self.cfg.zz_ext => self.zz_prepare(&slots),
            (Some("EXECUTE"), _) if self.cfg.zz_ext => self.zz_execute(),
            (Some("DELAYED-PUNCH"), _) if self.cfg.zz_ext => self.zz_delayed_punch(&slots),
            (Some("RELEASE-ALL-FINGERS"), _) if self.cfg.zz_ext => self.zz_release_all_fingers(),
            got => Err(unimpl(format!("manual request {got:?}"))),
        }
    }
}

/// `~f`/`~a` single-float display for the touch signals.
/// `direction=` (`motor.lisp:894`): two directions are equal if within π/4.
pub(super) fn direction_eq(d1: f32, d2: f32) -> bool {
    let delta = (d1 - d2).abs();
    (std::f32::consts::FRAC_PI_4 > delta) || (delta > 7.0 * std::f32::consts::PI / 4.0)
}

/// `minjerk-dist` (`motor.lisp:1256`): distance moved at time `tm`, given max
/// distance `a` and total move time `d`.
pub(super) fn minjerk_dist(tm: f32, a: f32, d: f32) -> f32 {
    if tm >= d {
        a
    } else if tm <= 0.0 || d <= 0.0 {
        0.0
    } else {
        let td = tm / d;
        a * (10.0 * td.powi(3) - 15.0 * td.powi(4) + 6.0 * td.powi(5))
    }
}

/// `pm-pixels-to-angle` (`device-interface.lisp:743`) at the default viewing
/// distance (`ppi × viewing-distance` = 72 × 15): the visual angle in degrees
/// subtended by `pixels`.
pub(super) fn pm_pixels_to_angle(pixels: f32) -> f32 {
    const DIST: f32 = 72.0 * 15.0;
    (2.0 * (pixels / 2.0 / DIST).atan()).to_degrees()
}

/// `approach-width` (`vision.lisp:3717`): the effective width in pixels of a
/// `w` × `h` target seen from approach angle `theta` (radians).
pub(super) fn approach_width(w: f32, h: f32, theta: f32) -> f32 {
    use std::f32::consts::{FRAC_PI_2, PI};
    let critical = h.atan2(w);
    let mut theta = theta.abs();
    if theta > FRAC_PI_2 {
        theta = PI - theta;
    }
    if theta == 0.0 {
        w
    } else if theta == FRAC_PI_2 {
        h
    } else if theta <= critical {
        w / theta.cos()
    } else {
        h / (FRAC_PI_2 - theta).cos()
    }
}

/// Distribute `m` references evenly from `end` back toward `start`
/// (`even-references`, commands/dm-commands.lisp:639). Integer ms.
pub(super) fn even_references(start: i64, end: i64, m: usize) -> Vec<i64> {
    if m == 0 {
        return Vec::new();
    }
    let n = m as f64;
    let decrement = (end - start) as f64 / n;
    let mut time = end as f64;
    let mut out = Vec::with_capacity(m);
    for _ in 0..m {
        time -= decrement;
        out.push(time.round() as i64);
    }
    out
}
