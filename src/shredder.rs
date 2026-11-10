//! The shredder, pinned to the window's bottom-right corner. A removed card flies (`matched`)
//! into its mouth, sinks through the slot, and comes out the bottom as strips that drop into
//! the bin and fade.

use rune::{Color, El, Spring, SpringPair, TextStyle, Transition, el, row, spacer, text};

use crate::card::{self, Ink};
use crate::store::Link;

const SW: f32 = 150.0;
/// Room above the slot for the card going in.
const MOUTH_H: f32 = 70.0;
const HEAD_H: f32 = 48.0;
const LIP: f32 = 10.0;
const BIN_W: f32 = SW - 24.0;
const BIN_H: f32 = 56.0;
const STRIPS: usize = 8;

/// What the shredder is doing.
pub enum Stage<'a> {
    Idle,
    /// The card is flying in and settling in the mouth.
    Feeding(&'a Link),
    /// The card is sinking through the slot and strips are coming out.
    Shredding(&'a Link),
}

pub fn shredder(stage: Stage, ink: &Ink) -> El {
    // Clipped only while shredding: a flying card keeps its destination's clips, so the mouth
    // stays open while the card flies in (the head hides its lower part).
    let mut mouth = el().key("mouth").absolute(0.0, 0.0).size(SW, MOUTH_H);
    if matches!(stage, Stage::Shredding(_)) {
        mouth = mouth.clip_rounded(0.5);
    }
    let mut bin = el()
        .key("bin")
        .absolute((SW - BIN_W) / 2.0, MOUTH_H - LIP + HEAD_H - 8.0)
        .size(BIN_W, BIN_H)
        .radius(10.0)
        .fill(ink.bg)
        .border(1.0, ink.line)
        .clip_rounded(10.0);
    let busy = !matches!(stage, Stage::Idle);
    match stage {
        Stage::Idle => {}
        Stage::Feeding(link) | Stage::Shredding(link) => {
            let shredding = matches!(stage, Stage::Shredding(_));
            let (w, h) = card::MINI;
            // Feeding: the card's top two thirds stand above the slot. Shredding: it sinks in.
            let y = if shredding { MOUTH_H + 4.0 } else { MOUTH_H - h * 0.62 };
            let mut paper = card::mini(link, ink, 0.6);
            if !shredding {
                paper = paper.matched(card::flight(link.id));
            }
            mouth = mouth.child(
                el().key(format!("intake-{}", link.id))
                    .absolute((SW - w) / 2.0, 0.0)
                    .offset(0.0, y)
                    .motion(Spring::new(0.95, 0.0))
                    .child(paper),
            );
            if shredding {
                bin = strips(bin, link, ink);
            }
        }
    }

    let lamp = el().size(6.0, 6.0).radius(3.0);
    let lamp = if busy { card::lit(lamp, ink, 0.9) } else { lamp.fill(ink.glow.alpha(0.12)) };
    let head = el()
        .key("shredder-head")
        .absolute(0.0, MOUTH_H - LIP)
        .size(SW, HEAD_H)
        .radius(14.0)
        .gradient(ink.raised, ink.card)
        .border(1.0, ink.line)
        .shadow(12.0, 26.0, Color::BLACK.alpha(0.6))
        .child(el().absolute(18.0, LIP - 3.0).size(SW - 36.0, 6.0).radius(3.0).fill(Color::BLACK).border(1.0, ink.line))
        .child(
            row()
                .absolute(16.0, 26.0)
                .w(SW - 32.0)
                .center_y()
                .gap(7.0)
                .child(lamp)
                .child(text("SHRED").font(TextStyle::new(9.5, 500.0).tracking(1.2)).color(ink.dim))
                .child(spacer())
                .child((0..4).fold(row().gap(3.0), |r, _| r.child(el().size(2.0, 10.0).radius(1.0).fill(ink.line)))),
        );
    el().key("shredder")
        .role("image")
        .label("Shredder")
        .size(SW, MOUTH_H - LIP + HEAD_H - 8.0 + BIN_H)
        .child(mouth)
        .child(bin)
        .child(head)
}

/// The card cut into strips: each comes down out of the head on its own slow spring, a little
/// askew, with bits of the card's lit keys on it, and falls away and fades when cleared.
fn strips(mut bin: El, link: &Link, ink: &Ink) -> El {
    let (w, _) = card::MINI;
    let sw = w / STRIPS as f32;
    let left = (BIN_W - w) / 2.0;
    let bits = card::key_bits(&link.url);
    for i in 0..STRIPS {
        let seed = bits.rotate_left(i as u32 * 5);
        let drop = 6.0 + (seed % 7) as f32;
        let turn = ((seed >> 3) % 9) as f32 * 0.012 - 0.048;
        let mut strip = el().size(sw - 1.5, 44.0).radius(1.0).fill(ink.card).border(1.0, ink.line);
        for k in 0..3 {
            if seed >> (k * 4) & 1 == 1 {
                strip = strip.child(card::lit(el().absolute((sw - 6.0) / 2.0 - 0.75, 6.0 + k as f32 * 12.0).size(5.0, 5.0).radius(1.0), ink, 0.6));
            }
        }
        let enter = Spring::new(0.75 + i as f32 * 0.04, 0.0);
        let exit = Spring::new(0.6, 0.0);
        bin = bin.child(
            el().key(format!("strip-{}-{i}", link.id))
                .absolute(left + i as f32 * sw, drop - 10.0)
                .rotate(turn)
                .transition(Transition::asymmetric(Transition::offset(0.0, -60.0), Transition::offset(0.0, 40.0).combined(Transition::opacity())).springs(SpringPair::new(enter, exit)))
                .child(strip),
        );
    }
    bin
}
