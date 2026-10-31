//! The grown-ups' door: a sum a five-year-old can't do yet, before
//! anything a grown-up should decide (more time, settings, reports).
//!
//! It keeps small hands out, not determined ones: real parental controls
//! belong to Sekka itself.

use aizome_kit::prelude::*;
use aizome_kit::{column, row};

/// The question, and the answer typed so far.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Gate {
    pub a: u32,
    pub b: u32,
    pub answer: String,
    pub wrong: bool,
}

impl Gate {
    /// A times table question from `seed` (6 × 6 to 9 × 9).
    pub fn new(seed: u64) -> Gate {
        let a = 6 + (seed % 4) as u32;
        let b = 6 + ((seed / 4) % 4) as u32;
        Gate { a, b, answer: String::new(), wrong: false }
    }

    pub fn question(&self) -> String {
        format!("What is {} × {}?", self.a, self.b)
    }

    pub fn right(&self) -> bool {
        self.answer.trim().parse::<u32>().ok() == Some(self.a * self.b)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateMsg {
    Answer(String),
    Submit,
    Cancel,
}

/// The sheet: what it's for, the question, the answer.
pub fn view(gate: &Gate, purpose: &str) -> Element<GateMsg> {
    let field = text_field(&gate.answer)
        .placeholder("Answer")
        .on_change(GateMsg::Answer)
        .on_submit(GateMsg::Submit)
        .error(gate.wrong.then_some("That's not it."))
        .id("gate-answer")
        .width(160.0);
    column![
        text("For grown-ups").size(20.0).bold(),
        text(purpose.to_owned()).role(Role::Text2).wrap(),
        row![text(gate.question()).size(16.0).semibold(), field].spacing(12.0),
        row![
            spacer(),
            button("Cancel").cancel().on_press(GateMsg::Cancel),
            button("Continue").default().on_press(GateMsg::Submit)
        ]
        .spacing(10.0),
    ]
    .spacing(14.0)
    .padding(24.0)
    .width(420.0)
    .into()
}
