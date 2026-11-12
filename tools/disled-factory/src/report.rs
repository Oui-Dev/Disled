// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The production log, one CSV row per board, appended as soon as the board
//! is done.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const HEADER: &str = "date,uid,firmware,signature,board_revision,colors,light_sensor,lux,\
                      touch_idle,touch_baseline,touch,touch_peak_permille,result,note";

/// One check of a board.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Check {
    #[default]
    NotRun,
    Pass,
    Fail,
}

impl From<bool> for Check {
    fn from(passed: bool) -> Self {
        if passed { Check::Pass } else { Check::Fail }
    }
}

impl Check {
    fn as_str(self) -> &'static str {
        match self {
            Check::NotRun => "",
            Check::Pass => "ok",
            Check::Fail => "fail",
        }
    }
}

/// What the log keeps of one board.
#[derive(Debug, Default)]
pub struct Row {
    /// UTC, RFC 3339.
    pub date: String,
    pub uid: String,
    /// As the lamp reports it.
    pub firmware: String,
    /// The firmware given to the tool was checked against the update key.
    pub signed: bool,
    pub board_revision: Option<u8>,
    pub colors: Check,
    /// The colors the operator saw wrong.
    pub wrong_colors: Vec<&'static str>,
    pub light_sensor: Check,
    pub lux_centi: Option<u32>,
    pub touch_idle: Check,
    pub touch_baseline: Option<u16>,
    pub touch: Check,
    pub touch_peak_permille: Option<i16>,
    /// Why the board failed outside the checks above, at the flash or the
    /// version check for instance.
    pub error: Option<String>,
    /// Facts that do not fail the board.
    pub remarks: Vec<String>,
}

impl Row {
    pub fn passed(&self) -> bool {
        self.error.is_none()
            && [self.colors, self.light_sensor, self.touch_idle, self.touch]
                .iter()
                .all(|check| *check == Check::Pass)
    }

    fn to_csv(&self) -> String {
        let colors = match self.colors {
            Check::Fail if !self.wrong_colors.is_empty() => {
                format!("fail: {}", self.wrong_colors.join(" "))
            }
            check => check.as_str().to_owned(),
        };
        let note: Vec<&str> = self
            .error
            .iter()
            .chain(&self.remarks)
            .map(String::as_str)
            .collect();
        [
            self.date.clone(),
            self.uid.clone(),
            self.firmware.clone(),
            (if self.signed { "checked" } else { "none" }).to_owned(),
            optional(self.board_revision),
            colors,
            self.light_sensor.as_str().to_owned(),
            self.lux_centi
                .map(|lux| format!("{}.{:02}", lux / 100, lux % 100))
                .unwrap_or_default(),
            self.touch_idle.as_str().to_owned(),
            optional(self.touch_baseline),
            self.touch.as_str().to_owned(),
            optional(self.touch_peak_permille),
            (if self.passed() { "PASS" } else { "FAIL" }).to_owned(),
            note.join("; "),
        ]
        .iter()
        .map(|field| quote(field))
        .collect::<Vec<_>>()
        .join(",")
    }
}

fn optional(value: Option<impl ToString>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

fn quote(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

pub struct Report {
    path: PathBuf,
}

impl Report {
    /// Opens the log; a new or empty file gets the header first.
    pub fn open(path: &Path) -> Result<Self, String> {
        let cannot = |e: std::io::Error| format!("cannot write {}: {e}", path.display());
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(cannot)?;
        if file.metadata().map_err(cannot)?.len() == 0 {
            writeln!(file, "{HEADER}").map_err(cannot)?;
        }
        Ok(Self {
            path: path.to_owned(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, row: &Row) -> Result<(), String> {
        let cannot = |e: std::io::Error| format!("cannot write {}: {e}", self.path.display());
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(cannot)?;
        writeln!(file, "{}", row.to_csv()).map_err(cannot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing() -> Row {
        Row {
            date: "2026-10-21T04:30:12Z".into(),
            uid: "F1B9ABCD5A9BBD15E339E339".into(),
            firmware: "0.3.0".into(),
            signed: true,
            board_revision: Some(4),
            colors: Check::Pass,
            light_sensor: Check::Pass,
            lux_centi: Some(24531),
            touch_idle: Check::Pass,
            touch_baseline: Some(2618),
            touch: Check::Pass,
            touch_peak_permille: Some(312),
            ..Row::default()
        }
    }

    #[test]
    fn a_passing_board() {
        let row = passing();
        assert!(row.passed());
        assert_eq!(
            row.to_csv(),
            "2026-10-21T04:30:12Z,F1B9ABCD5A9BBD15E339E339,0.3.0,checked,4,ok,ok,245.31,ok,2618,ok,312,PASS,"
        );
        assert_eq!(HEADER.split(',').count(), row.to_csv().split(',').count());
    }

    #[test]
    fn a_failing_board_says_why() {
        let row = Row {
            colors: Check::Fail,
            wrong_colors: vec!["red", "blue"],
            remarks: vec!["fault before the last reset: mcause 0x7".into()],
            ..passing()
        };
        assert!(!row.passed());
        assert!(row.to_csv().ends_with(
            ",fail: red blue,ok,245.31,ok,2618,ok,312,FAIL,fault before the last reset: mcause 0x7"
        ));
    }

    #[test]
    fn a_board_that_stopped_early() {
        let row = Row {
            date: "2026-10-21T04:31:00Z".into(),
            error: Some("wchisp: Operation timed out, \"erase\"".into()),
            ..Row::default()
        };
        assert!(!row.passed());
        assert_eq!(
            row.to_csv(),
            "2026-10-21T04:31:00Z,,,none,,,,,,,,,FAIL,\"wchisp: Operation timed out, \"\"erase\"\"\""
        );
    }
}
