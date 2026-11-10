//! The card printer: an old-school receipt printer the new card feeds out of.
//!
//! The card sits in a clipped paper area whose bottom edge is the printer's slot, pushed up by
//! `visible` points on a rolling spring. Its top (number, address, note) shows from the start,
//! so you type straight onto it; every character feeds it up a little, a backspace rolls it
//! back, and printing pushes it all the way out. The app then swaps it for the saved card,
//! which flies (`matched`) into its folder. The category keys and PRINT sit on the body.

use rune::{Color, El, Spring, el, row};

use crate::card::{self, Ink};

pub const PW: f32 = 336.0;
/// Room above the slot: enough for the whole card when it's out.
const PAPER_H: f32 = card::H + 16.0;
const BODY_H: f32 = 76.0;
/// How far the body rises over the bottom of the paper area.
const LIP: f32 = 12.0;
pub const PH: f32 = PAPER_H - LIP + BODY_H;

/// How much of the card shows for the text typed so far: its top when empty, a little more
/// per character and a step for a category, never quite all of it until it prints.
pub fn feed(chars: usize, has_category: bool) -> f32 {
    let steps = chars as f32 * 2.4 + if has_category { 16.0 } else { 0.0 };
    (card::BLANK_TOP + LIP + 6.0 + steps).min(card::H - 24.0)
}

/// All of the card, clear of the slot.
pub const OUT: f32 = card::H + 8.0;

/// `job` keys the sheet, so a new blank one mounts after each print and the printed one can
/// fly away. `keys` is the row of buttons on the body.
pub fn printer(job: u64, paper: El, visible: f32, keys: El, ink: &Ink) -> El {
    let roll = Spring::new(0.42, 0.2);
    let sheet = el()
        .key(format!("sheet-{job}"))
        .absolute((PW - card::W) / 2.0, PAPER_H)
        .offset(0.0, -visible)
        .motion(roll)
        .child(paper);
    let paper_area = el().key("paper").absolute(0.0, 0.0).size(PW, PAPER_H).clip_rounded(0.5).child(sheet);
    let body = el()
        .key("printer-body")
        .absolute(0.0, PAPER_H - LIP)
        .size(PW, BODY_H)
        .radius(18.0)
        .gradient(ink.raised, ink.card)
        .border(1.0, ink.line)
        .shadow(14.0, 30.0, Color::BLACK.alpha(0.6))
        // The slot the card comes out of.
        .child(el().absolute(30.0, LIP - 3.0).size(PW - 60.0, 6.0).radius(3.0).fill(Color::BLACK).border(1.0, ink.line))
        .child(row().absolute(16.0, 30.0).w(PW - 32.0).center_y().child(keys));
    el().key("printer").size(PW, PH).child(paper_area).child(body)
}

/// A key on the printer's body. `lit` glows it; `primary` is the white PRINT key.
pub fn key(label: El, lit: bool, primary: bool, ink: &Ink) -> El {
    let k = row().h(30.0).px(8.0).gap(5.0).center().radius(8.0).child(label);
    if primary {
        k.px(12.0).fill(ink.glow).shadow(0.0, 14.0, ink.glow.alpha(0.25)).hover(Color::BLACK.alpha(0.08))
    } else if lit {
        k.fill(ink.glow.alpha(0.08)).border(1.0, ink.glow.alpha(0.45)).shadow(0.0, 12.0, ink.glow.alpha(0.12))
    } else {
        k.fill(ink.bg.alpha(0.5)).border(1.0, ink.line).hover(ink.glow.alpha(0.05))
    }
}
