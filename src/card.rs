//! The look, after Inspora's "Retro CRT": near-black, dark cards with hairline edges, a faint
//! isometric grid, and the only light a white glow from keys that are lit.
//!
//! Each link's card shows a small keypad on that grid. Which keys are lit comes from the
//! link's host, mirrored like an identicon, so a site keeps its pattern. Hovering a card
//! brightens its keys.

use std::f32::consts::FRAC_PI_6;

use rune::{Color, El, TextStyle, Tx, col, el, oklch, oklcha, row, spacer, text};

use crate::store::{self, Category, Link};

pub const W: f32 = 228.0;
pub const H: f32 = 318.0;
const ART_H: f32 = 168.0;
const PAD: f32 = 20.0;
const RADIUS: f32 = 16.0;

/// The palette: almost black, greys, hairlines, and white light.
pub struct Ink {
    pub bg: Color,
    pub card: Color,
    pub raised: Color,
    pub line: Color,
    pub grid: Color,
    pub text: Color,
    pub muted: Color,
    pub dim: Color,
    pub glow: Color,
    pub error: Color,
}

impl Ink {
    pub fn new() -> Ink {
        Ink {
            bg: oklch(0.135, 0.0, 0.0),
            card: oklch(0.175, 0.0, 0.0),
            raised: oklch(0.205, 0.0, 0.0),
            line: oklcha(1.0, 0.0, 0.0, 0.09),
            grid: oklcha(1.0, 0.0, 0.0, 0.045),
            text: oklch(0.9, 0.0, 0.0),
            muted: oklch(0.64, 0.0, 0.0),
            dim: oklch(0.48, 0.0, 0.0),
            glow: oklch(0.97, 0.0, 0.0),
            error: oklch(0.72, 0.13, 30.0),
        }
    }
}

/// A lit key: white, with light spilling round it. `strength` 0..1 sets how bright.
pub fn lit(e: El, ink: &Ink, strength: f32) -> El {
    e.fill(ink.glow.alpha(0.3 + 0.67 * strength)).shadow(0.0, 2.0 + 16.0 * strength, ink.glow.alpha(0.08 + 0.55 * strength))
}

/// A key that is off: a hairline square.
fn unlit(e: El, ink: &Ink) -> El {
    e.border(1.0, ink.line)
}

/// A category's icon: a 5×5 pixel glyph. VM is a prompt, Design a diamond, Content lines of
/// text, Unsorted a single dot.
fn glyph_bits(c: Option<Category>) -> [u8; 5] {
    match c {
        Some(Category::Vm) => [0b10000, 0b01000, 0b00100, 0b01000, 0b10111],
        Some(Category::Design) => [0b00100, 0b01010, 0b10001, 0b01010, 0b00100],
        Some(Category::Content) => [0b11111, 0b00000, 0b11110, 0b00000, 0b11100],
        None => [0b00000, 0b00000, 0b00100, 0b00000, 0b00000],
    }
}

/// The glyph at `px` points per pixel, lit at `strength`.
pub fn glyph(c: Option<Category>, px: f32, ink: &Ink, strength: f32) -> El {
    let gap = (px * 0.25).max(1.0);
    let mut g = col().gap(gap);
    for bits in glyph_bits(c) {
        let mut r = row().gap(gap);
        for i in (0..5).rev() {
            let cell = el().size(px, px).radius(px * 0.18);
            r = r.child(if bits >> i & 1 == 1 { lit(cell, ink, strength) } else { cell });
        }
        g = g.child(r);
    }
    g
}

/// The app's mark: one lit key on a keycap.
pub fn logo(ink: &Ink) -> El {
    el().size(22.0, 22.0)
        .radius(6.0)
        .gradient(ink.raised, ink.card)
        .border(1.0, ink.line)
        .center()
        .child(lit(el().size(8.0, 8.0).radius(2.0), ink, 0.9))
}

/// What a card's controls do, supplied by the app.
pub struct Actions<S> {
    pub open: fn(&mut S, &str, &mut Tx),
    pub remove: fn(&mut S, u64, &mut Tx),
    pub retag: fn(&mut S, u64),
    pub hover: fn(&mut S, u64, bool),
}

/// The category as a hairline pill with its glyph.
fn tag_pill(category: Option<Category>, ink: &Ink) -> El {
    pill(category, category.is_some(), false, ink)
}

/// A pill with a category's glyph and name. `bright` is a set category; `picked` lights the
/// whole pill, for the category picker on the new card.
pub fn pill(category: Option<Category>, bright: bool, picked: bool, ink: &Ink) -> El {
    let label = category.map_or("Untagged", Category::label);
    let p = row()
        .h(26.0)
        .px(9.0)
        .gap(6.0)
        .center_y()
        .radius(13.0)
        .child(glyph(category, 2.0, ink, if picked { 1.0 } else if bright { 0.5 } else { 0.1 }))
        .child(text(label).font(TextStyle::new(11.5, 500.0)).color(if picked { ink.glow } else if bright { ink.muted } else { ink.dim }));
    if picked {
        p.fill(ink.glow.alpha(0.08)).border(1.0, ink.glow.alpha(0.45)).shadow(0.0, 12.0, ink.glow.alpha(0.12))
    } else {
        p.border(1.0, ink.line)
    }
}

/// What's printed on a card. `title` is None for a card still being typed with no name yet.
pub struct Face<'a> {
    pub url: &'a str,
    pub title: Option<&'a str>,
    pub when: &'a str,
    /// The card's place in the whole collection, from 0, and the collection's size.
    pub n: usize,
    pub total: usize,
    pub strength: f32,
    pub glow_edge: bool,
}

/// The card itself, with `foot` along its bottom; no handlers.
pub fn face(f: &Face, foot: El, ink: &Ink) -> El {
    let host = store::host(f.url);
    let meta = if host.is_empty() { f.when.to_string() } else { format!("{} · {}", shorten(host, 26), f.when) };
    let title = match f.title {
        Some(t) => text(shorten(t, 60)).color(ink.text),
        None => text("Untitled").color(ink.dim),
    };
    let words = col()
        .w(W - 2.0 * PAD)
        .gap(8.0)
        .child(text(format!("{:02} / {:02}", f.n + 1, f.total)).font(TextStyle::new(11.5, 400.0).tracking(0.4)).color(ink.dim))
        .child(title.wrap().font(TextStyle::new(18.0, 500.0).lh(23.0).tracking(-0.2)))
        .child(text(meta).font(TextStyle::new(12.5, 400.0)).color(ink.muted));
    col()
        .size(W, H)
        .radius(RADIUS)
        .fill(ink.card)
        .border(1.0, if f.glow_edge { ink.glow.alpha(0.16) } else { ink.line })
        .shadow(12.0, 32.0, Color::BLACK.alpha(0.5))
        .clip_rounded(RADIUS)
        .child(art(f.url, W, ART_H, f.strength, ink))
        .child(col().w(W).grow(1.0).px(PAD).pb(PAD - 4.0).gap(12.0).child(words).child(spacer()).child(foot.w(W - 2.0 * PAD)))
}

/// A saved link's card. It flies (`matched`) between the grid, its folder and the printer.
pub fn card<S: 'static>(link: &Link, n: usize, total: usize, hot: bool, ink: &Ink, act: &Actions<S>) -> El {
    let id = link.id;
    let url = link.url.clone();
    let (remove, hover, open, retag) = (act.remove, act.hover, act.open, act.retag);
    let tag = tag_pill(link.category, ink)
        .key("tag")
        .role("button")
        .label(format!("Category: {}", link.category.map_or("none", Category::label)))
        .hover(ink.glow.alpha(0.06))
        .on_press(move |s: &mut S| retag(s, id));
    let foot = row().center_y().child(tag).child(spacer()).child(
        text("Shred")
            .key("remove")
            .role("button")
            .label("Shred")
            .font(TextStyle::new(11.0, 400.0))
            .color(ink.dim)
            .on_press_tx(move |s: &mut S, tx: &mut Tx| remove(s, id, tx)),
    );
    let when = ago(link.added);
    let f = Face { url: &link.url, title: Some(link.name()), when: &when, n, total, strength: if hot { 1.0 } else { 0.3 }, glow_edge: hot };
    face(&f, foot, ink)
        .key(format!("card-{id}"))
        .matched(flight(id))
        .role("link")
        .label(link.name())
        .on_hover_change(move |s: &mut S, inside: bool, _: &mut Tx| hover(s, id, inside))
        .on_press_tx(move |s: &mut S, tx: &mut Tx| open(s, &url, tx))
}

/// The top of a card in the printer, which shows first: its number, the address, the note.
pub const BLANK_TOP: f32 = 104.0;

/// A card in the printer, laid out top first so what's typed shows as soon as it's fed: the
/// number, `title` and `note` (the fields while typing, plain text once printed), then the
/// keypad printing as the address comes in, then the category.
pub struct Blank<'a> {
    pub counter: String,
    pub title: El,
    pub note: El,
    pub error: Option<&'a str>,
    pub url: &'a str,
    pub category: Option<Category>,
    pub strength: f32,
}

pub fn blank(b: Blank, ink: &Ink) -> El {
    let Blank { counter, title, note, error, url, category, strength } = b;
    let mut top = col()
        .w(W)
        .h(BLANK_TOP)
        .px(PAD)
        .pt(18.0)
        .gap(6.0)
        .child(text(counter).font(TextStyle::new(11.5, 400.0).tracking(0.4)).color(ink.dim))
        .child(el().h(2.0))
        .child(title.w(W - 2.0 * PAD))
        .child(note.w(W - 2.0 * PAD));
    if let Some(e) = error {
        top = top.child(text(e.to_string()).key("error").font(TextStyle::new(11.5, 400.0)).color(ink.error));
    }
    col()
        .size(W, H)
        .radius(RADIUS)
        .fill(ink.card)
        .border(1.0, ink.line)
        .shadow(12.0, 32.0, Color::BLACK.alpha(0.5))
        .clip_rounded(RADIUS)
        .child(top)
        .child(art(url, W, ART_H, strength, ink))
        .child(row().w(W).grow(1.0).px(PAD).center_y().child(tag_pill(category, ink)))
}

/// The id a link's card flies by, wherever it's shown.
pub fn flight(id: u64) -> String {
    format!("link-{id}")
}

/// A link's picture, `w` × `h`: the iso grid and its keypad, keys lit at `strength`.
fn art(url: &str, w: f32, h: f32, strength: f32, ink: &Ink) -> El {
    let k = w / W;
    let host = store::host(url);
    let seed = hash(host);
    let mut a = el().key("art").w(w).h(h).clip_rounded(0.5);
    // The grid: hairlines at ±30°, as an isometric floor. Each is long enough to cross the box.
    let len = (w * w + h * h).sqrt() * 1.4;
    let step = 26.0 * k;
    for i in -8..14 {
        let x = i as f32 * step * 1.7;
        for turn in [FRAC_PI_6, -FRAC_PI_6] {
            a = a.child(el().absolute(x - len / 2.0, h / 2.0).size(len, 1.0).fill(ink.grid).rotate(turn));
        }
    }
    // The keypad: 8 × 4 keys, mirrored left to right, tilted onto the grid.
    let key = 13.0 * k;
    let gap = 5.0 * k;
    let mut pad = col().gap(gap).padding(7.0 * k).radius(5.0 * k).fill(ink.raised).border(1.0, ink.line);
    for r in 0..4u32 {
        let mut line = row().gap(gap);
        for c in 0..8u32 {
            let m = if c < 4 { c } else { 7 - c };
            // About three keys in eight are lit.
            let on = !host.is_empty() && hash_mix(seed, r * 4 + m) % 8 < 3;
            let cell = el().size(key, key).radius(2.0 * k);
            line = line.child(if on { lit(cell, ink, strength) } else { unlit(cell, ink) });
        }
        pad = pad.child(line);
    }
    let pad_w = 8.0 * key + 7.0 * gap + 14.0 * k;
    let pad_h = 4.0 * key + 3.0 * gap + 14.0 * k;
    a.child(el().absolute((w - pad_w) / 2.0, (h - pad_h) / 2.0 + 6.0 * k).rotate(-FRAC_PI_6).child(pad))
}

const MINI_W: f32 = 92.0;
const MINI_H: f32 = 118.0;
/// A mini card's size.
pub const MINI: (f32, f32) = (MINI_W, MINI_H);

/// A small card for a folder: the picture and the title.
pub fn mini(link: &Link, ink: &Ink, strength: f32) -> El {
    col()
        .size(MINI_W, MINI_H)
        .radius(8.0)
        .fill(ink.card)
        .border(1.0, ink.line)
        .shadow(4.0, 14.0, Color::BLACK.alpha(0.55))
        .clip_rounded(8.0)
        .child(art(&link.url, MINI_W, 68.0, strength, ink))
        .child(
            text(shorten(link.name(), 30))
                .wrap()
                .w(MINI_W - 14.0)
                .mx(7.0)
                .mt(6.0)
                .font(TextStyle::new(8.5, 500.0).lh(10.5))
                .color(ink.text),
        )
}

/// Another stable number from `seed` for key `i`.
fn hash_mix(seed: u32, i: u32) -> u32 {
    let mut x = seed ^ i.wrapping_mul(0x9e37_79b9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x
}

/// A number standing for a link's lit keys, for things drawn to match its card.
pub fn key_bits(url: &str) -> u32 {
    hash(store::host(url))
}

fn hash(s: &str) -> u32 {
    // FNV-1a: stable across runs, unlike std's randomly seeded hasher.
    s.bytes().fold(0x811c_9dc5, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

pub fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_string() } else { format!("{}…", s.chars().take(max - 1).collect::<String>().trim_end()) }
}

/// `added` as "just now", "5m ago", "3h ago", "2d ago", "4w ago".
pub fn ago(added: u64) -> String {
    let s = store::now().saturating_sub(added);
    match s {
        0..60 => "just now".into(),
        60..3600 => format!("{}m ago", s / 60),
        3600..86_400 => format!("{}h ago", s / 3600),
        86_400..604_800 => format!("{}d ago", s / 86_400),
        _ => format!("{}w ago", s / 604_800),
    }
}
