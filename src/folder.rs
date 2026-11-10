//! A folder of links: a dark cover with a hairline edge and the category's glowing glyph, and
//! the newest cards tucked in behind it, peeking out of the right edge. On hover the cover eases
//! open, the cards slide out and fan, and their keys light up; a click opens the folder.

use rune::{Color, El, Spring, TextStyle, col, el, row, spacer, text};

use crate::card::{self, Ink};
use crate::store::{Category, Link};

const FW: f32 = 138.0;
const FH: f32 = 180.0;
/// The folder plus the room its cards fan into.
const AREA_W: f32 = 220.0;
const AREA_H: f32 = 204.0;
const TOP: f32 = 12.0;
const RADIUS: f32 = 14.0;

pub fn name(c: Option<Category>) -> &'static str {
    c.map_or("Unsorted", Category::label)
}

pub fn key(c: Option<Category>) -> &'static str {
    c.map_or("unsorted", Category::key)
}

/// What a folder does when used, supplied by the app.
pub struct Actions<S> {
    pub open: fn(&mut S, Option<Category>),
    pub hover: fn(&mut S, Option<Category>, bool),
}

/// `links` are the folder's links, newest first; the first three peek out.
pub fn folder<S: 'static>(c: Option<Category>, links: &[&Link], hovered: bool, ink: &Ink, act: &Actions<S>) -> El {
    let bouncy = Spring::new(0.5, 0.22);
    let count = links.len();
    let mut area = el()
        .key(format!("folder-{}", key(c)))
        .role("button")
        .label(format!("{} folder, {count} links", name(c)))
        .size(AREA_W, AREA_H)
        .on_press({
            let open = act.open;
            move |s: &mut S| open(s, c)
        })
        .on_hover_change({
            let hover = act.hover;
            move |s: &mut S, inside: bool, _: &mut rune::Tx| hover(s, c, inside)
        })
        // The back of the folder, a little offset so its edge shows.
        .child(el().absolute(6.0, TOP - 4.0).size(FW, FH + 4.0).radius(RADIUS).fill(ink.card).border(1.0, ink.line));

    // Oldest first, so the newest card sits on top and furthest out.
    let shown: Vec<&Link> = links.iter().take(3).copied().collect();
    for (k, link) in shown.iter().enumerate().rev() {
        let k = k as f32;
        let (x, y, turn) = if hovered {
            (FW - 4.0 - 34.0 * k, TOP + 8.0 + 5.0 * k, 0.05 - 0.05 * k)
        } else {
            (FW - 50.0 - 12.0 * k, TOP + 16.0 + 6.0 * k, 0.04 + 0.05 * k)
        };
        area = area.child(
            el().key(format!("peek-{}", link.id))
                .matched(card::flight(link.id))
                .absolute(0.0, 0.0)
                .offset(x, y)
                .rotate(turn)
                .motion(bouncy)
                .child(card::mini(link, ink, if hovered { 1.0 } else { 0.25 })),
        );
    }

    let rule = || el().w_full().h(1.0).fill(ink.line);
    let cover = col()
        .key("cover")
        .absolute(0.0, TOP)
        .size(if hovered { FW - 16.0 } else { FW }, FH)
        .motion(bouncy)
        .radius(RADIUS)
        .gradient(ink.raised, ink.card)
        .border(1.0, if hovered { ink.glow.alpha(0.18) } else { ink.line })
        .shadow(10.0, 28.0, Color::BLACK.alpha(0.6))
        .padding(16.0)
        .child(row().child(card::glyph(c, 4.0, ink, if hovered { 1.0 } else { 0.45 })))
        .child(spacer())
        .child(text(format!("{count:02}")).font(TextStyle::new(11.0, 400.0).tracking(0.4)).color(ink.dim))
        .child(el().h(4.0))
        .child(text(name(c)).font(TextStyle::new(16.0, 500.0)).color(ink.text))
        .child(el().h(12.0))
        .child(rule())
        .child(el().h(3.0))
        .child(rule());
    area.child(cover)
}
