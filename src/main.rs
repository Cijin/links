//! Links: save links to interesting websites, find them again, open them in the browser.
//! Links sit in folders by category; each is a dark card with a glowing keypad drawn from its
//! host, after Inspora's "Retro CRT". New cards feed out of a little printer as you type.
//!
//! `cargo run --release` opens the window.
//! `cargo run --release -- --snap links.png --scene sample` renders it headless.

mod card;
mod folder;
mod printer;
mod shredder;
mod store;

use rune::tok::type_;
use rune::{AppSpec, Cx, El, Key, KeyEvent, ScrollSpec, TextEdit, TextStyle, Tx, col, el, row, spacer, text, text_input};
use card::{Actions, Ink};
use store::{Category, Link};

struct State {
    links: Vec<Link>,
    url: TextEdit,
    title: TextEdit,
    search: TextEdit,
    /// The category the next saved link gets. It stays picked, for saving several in a row.
    category: Option<Category>,
    /// The shelf of folders, or one folder open.
    mode: Mode,
    /// The folder under the pointer, whose cards fan out.
    hovered: Option<Option<Category>>,
    /// The card under the pointer, whose keys glow brighter.
    hot: Option<u64>,
    /// The card on its way out of the printer.
    print: Print,
    /// The card on its way through the shredder.
    shred: Shred,
    /// Counts printed cards, so each print gets a fresh sheet in the printer.
    job: u64,
    /// Why the last add was refused, shown under the form until the next edit.
    error: Option<&'static str>,
    /// False in headless sample scenes, so snapshots never touch the real file.
    persist: bool,
}

/// How long a printed card sits fully out of the printer before it flies to its folder.
const EJECT_SECS: f64 = 0.7;

enum Print {
    /// The printer holds the card being typed.
    Idle,
    /// A saved card rolling out; at `due` it lands in the collection and flies to its folder.
    Ejecting { link: Link, due: f64 },
}

/// How long a removed card takes to fly into the shredder's mouth, then to go through it.
const FEED_SECS: f64 = 0.6;
const SHRED_SECS: f64 = 1.2;

enum Shred {
    Idle,
    /// The removed card flying into the shredder; at `due` it starts going through.
    Feeding { link: Link, due: f64 },
    /// The card sinking through the slot as strips come out; at `due` the strips fall away.
    Shredding { link: Link, due: f64 },
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Shelf,
    /// The folder for this category; None is Unsorted.
    Folder(Option<Category>),
}

impl State {
    fn next_id(&self) -> u64 {
        self.links.iter().chain(self.printing()).map(|l| l.id).max().map_or(1, |m| m + 1)
    }

    fn printing(&self) -> Option<&Link> {
        match &self.print {
            Print::Ejecting { link, .. } => Some(link),
            Print::Idle => None,
        }
    }

    /// Puts the card that's rolling out into the collection. The printer gets a fresh sheet,
    /// and the printed one, now gone from it, flies to wherever the link is shown.
    fn land(&mut self) {
        if let Print::Ejecting { link, .. } = std::mem::replace(&mut self.print, Print::Idle) {
            self.links.insert(0, link);
            self.job += 1;
            self.save();
        }
    }

    fn save(&self) {
        if self.persist {
            store::save(&self.links);
        }
    }

    fn add(&mut self, tx: &mut Tx) {
        // A second print before the first has flown finishes the first at once.
        self.land();
        let Some(url) = store::normalize(&self.url.text) else {
            self.error = Some(if self.url.text.trim().is_empty() { "Paste or type a link first" } else { "That doesn't look like a web address" });
            return;
        };
        if self.links.iter().any(|l| l.url == url) {
            self.error = Some("Already saved");
            return;
        }
        let link = Link { id: self.next_id(), url, title: self.title.text.trim().to_string(), added: store::now(), category: self.category };
        self.url.clear();
        self.title.clear();
        self.url.focused = true;
        self.error = None;
        let due = tx.clock() + EJECT_SECS;
        tx.wake_at(due);
        self.print = Print::Ejecting { link, due };
    }

    /// Takes the link out of the collection at once (and off disk) and sends its card to the
    /// shredder, which only animates it. A card already in the shredder is dropped.
    fn remove(&mut self, id: u64, tx: &mut Tx) {
        let Some(i) = self.links.iter().position(|l| l.id == id) else { return };
        let link = self.links.remove(i);
        self.save();
        if self.hot == Some(id) {
            self.hot = None;
        }
        let due = tx.clock() + FEED_SECS;
        tx.wake_at(due);
        self.shred = Shred::Feeding { link, due };
    }

    /// Moves the shredder on when its timer is due; true when something changed.
    fn step_shred(&mut self, now: f64, tx: &mut Tx) -> bool {
        match std::mem::replace(&mut self.shred, Shred::Idle) {
            Shred::Feeding { link, due } if now >= due => {
                let due = now + SHRED_SECS;
                tx.wake_at(due);
                self.shred = Shred::Shredding { link, due };
                true
            }
            Shred::Shredding { due, .. } if now >= due => true,
            other => {
                self.shred = other;
                false
            }
        }
    }

    fn retag(&mut self, id: u64) {
        if let Some(l) = self.links.iter_mut().find(|l| l.id == id) {
            l.category = Category::cycle(l.category);
        }
        self.save();
    }

    fn in_folder(&self, c: Option<Category>) -> Vec<&Link> {
        self.links.iter().filter(|l| l.category == c).collect()
    }

    fn open_folder(&mut self, c: Option<Category>) {
        self.mode = Mode::Folder(c);
        self.hovered = None;
        // Links saved from inside a folder go into it.
        self.category = c;
    }

    /// The links the open folder and the search match, each with its stamp number (its place in the whole list).
    fn visible(&self) -> Vec<(usize, &Link)> {
        let q = self.search.text.trim().to_lowercase();
        self.links
            .iter()
            .enumerate()
            .filter(|(_, l)| match self.mode {
                Mode::Shelf => true,
                Mode::Folder(c) => l.category == c,
            })
            .filter(|(_, l)| q.is_empty() || l.title.to_lowercase().contains(&q) || l.url.to_lowercase().contains(&q))
            .collect()
    }
}

fn init() -> State {
    init_with(store::load())
}

fn init_with(links: Vec<Link>) -> State {
    let mut url = TextEdit::new("");
    url.focused = true;
    State { links, url, title: TextEdit::new(""), search: TextEdit::new(""), category: None, mode: Mode::Shelf, hovered: None, hot: None, print: Print::Idle, shred: Shred::Idle, job: 0, error: None, persist: true }
}

/// The spacing scale everything sits on.
const S2: f32 = 8.0;
const S3: f32 = 12.0;
const S6: f32 = 24.0;
const S8: f32 = 32.0;
/// The window's toolbar, level with the traffic lights; content starts clear of them.
const TOOLBAR_H: f32 = 52.0;
const LIGHTS_W: f32 = 84.0;
const SIDEBAR_W: f32 = printer::PW + 2.0 * S6;

/// A small hairline button.
fn button(key: &'static str, label: &'static str, ink: &Ink, press: impl Fn(&mut State) + 'static) -> El {
    row()
        .key(key)
        .role("button")
        .label(label)
        .h(30.0)
        .px(S3)
        .center()
        .radius(8.0)
        .border(1.0, ink.line)
        .hover(ink.glow.alpha(0.05))
        .on_press(press)
        .child(text(label).font(TextStyle::new(12.0, 500.0)).color(ink.muted))
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let ink = Ink::new();
    let toolbar = row()
        .key("toolbar")
        .w_full()
        .h(TOOLBAR_H)
        .pl(LIGHTS_W)
        .pr(S6)
        .gap(S3)
        .center_y()
        .child(card::logo(&ink))
        .child(text("Links").font(TextStyle::new(15.0, 600.0).tracking(-0.1)).color(ink.text))
        .child(spacer())
        .child(
            row()
                .key("search-field")
                .w(260.0)
                .h(30.0)
                .px(S3)
                .radius(8.0)
                .fill(ink.card)
                .border(1.0, ink.line)
                .center_y()
                .child(
                    text_input(&s.search, TextStyle::new(12.5, 400.0))
                        .key("search")
                        .single_line()
                        .grow(1.0)
                        .color(ink.text)
                        .placeholder("Search", ink.dim)
                        .on_edit(|s: &mut State| &mut s.search),
                ),
        );
    let sidebar = col()
        .key("sidebar")
        .w(SIDEBAR_W)
        .h_full()
        .px(S6)
        .pb(S8)
        .justify_end()
        .child(printer_view(s, &ink));
    let main = col().key("main").grow(1.0).h_full().min_w(0.0).px(S6).pt(S6).gap(S6).child(section_head(s, &ink)).child(main_body(s, &ink));
    col()
        .key("root")
        .w_full()
        .h_full()
        .fill(ink.bg)
        .on_paste(|s: &mut State, pasted: &str, _| {
            // Pasting with no field focused drops the link onto the new card.
            s.url.replace_text(pasted.trim());
            s.url.focused = true;
            s.error = None;
        })
        .child(toolbar)
        .child(el().w_full().h(1.0).fill(ink.line))
        .child(row().w_full().grow(1.0).min_h(0.0).child(sidebar).child(el().w(1.0).h_full().fill(ink.line)).child(main))
        .child(shredder_view(s, &ink))
}

/// The title over the main area: Folders, an open folder, or search results.
fn section_head(s: &State, ink: &Ink) -> El {
    let title = |t: String| text(t).font(TextStyle::new(22.0, 500.0).tracking(-0.3)).color(ink.text);
    let count = |n: usize| text(if n == 1 { "1 link".to_string() } else { format!("{n} links") }).font(TextStyle::new(12.5, 400.0)).color(ink.dim);
    let q = s.search.text.trim();
    let head = row().key("section").w_full().h(32.0).gap(S3).center_y();
    match s.mode {
        _ if !q.is_empty() => head.child(title(format!("Results for “{q}”"))).child(count(s.visible().len())),
        Mode::Shelf => head.child(title("Folders".into())).child(count(s.links.len())),
        Mode::Folder(c) => head
            .child(button("back", "← Folders", ink, |s| s.mode = Mode::Shelf))
            .child(el().w(S2))
            .child(card::glyph(c, 3.0, ink, 0.9))
            .child(title(folder::name(c).into()))
            .child(count(s.in_folder(c).len())),
    }
}

fn main_body(s: &State, ink: &Ink) -> El {
    let searching = !s.search.text.trim().is_empty();
    match s.mode {
        Mode::Shelf if !searching => shelf(s, ink),
        _ => sheet(s, ink),
    }
}

/// The printer with its card: the one being typed, or the one rolling out.
fn printer_view(s: &State, ink: &Ink) -> El {
    let counter = format!("{:02} / {:02}", 1, s.links.len() + 1);
    let mut keys = row().w_full().gap(6.0).center_y();
    for c in Category::ALL {
        let on = s.category == Some(c);
        let label = row().gap(5.0).child(card::glyph(Some(c), 2.0, ink, if on { 1.0 } else { 0.4 })).child(text(c.label()).font(TextStyle::new(11.5, 500.0)).color(if on { ink.glow } else { ink.muted }));
        // Pressing the lit one again leaves the link uncategorized.
        keys = keys.child(
            printer::key(label, on, false, ink)
                .key(format!("pick-{}", c.key()))
                .role("button")
                .label(c.label())
                .checked(on)
                .on_press(move |s: &mut State| s.category = if on { None } else { Some(c) }),
        );
    }
    keys = keys.child(spacer()).child(
        printer::key(text("Print").font(TextStyle::new(12.0, 600.0)).color(ink.bg), false, true, ink)
            .key("add")
            .role("button")
            .label("Print")
            .on_press_tx(|s: &mut State, tx: &mut Tx| s.add(tx)),
    );
    let title_font = TextStyle::new(17.0, 500.0).lh(22.0).tracking(-0.2);
    let note_font = TextStyle::new(12.5, 400.0).lh(17.0);
    match s.printing() {
        Some(link) => {
            let note = format!("{} · just now", store::host(&link.url));
            let paper = card::blank(
                card::Blank {
                    counter,
                    title: text(link.name().to_string()).font(title_font).color(ink.text),
                    note: text(note).font(note_font).color(ink.muted),
                    error: None,
                    url: &link.url,
                    category: link.category,
                    strength: 1.0,
                },
                ink,
            )
            .matched(card::flight(link.id));
            printer::printer(s.job, paper, printer::OUT, keys, ink)
        }
        None => {
            // You type straight onto the card: the address is its title, the note under it.
            let url = text_input(&s.url, title_font)
                .key("url")
                .single_line()
                .color(ink.text)
                .placeholder("Paste a link", ink.dim)
                .on_edit(|s: &mut State| &mut s.url)
                .on_input_change(|s: &mut State, _| s.error = None)
                .on_submit(|s: &mut State, tx| s.add(tx));
            let note = text_input(&s.title, note_font)
                .key("title")
                .single_line()
                .color(ink.muted)
                .placeholder("Add a title or note", ink.dim)
                .on_edit(|s: &mut State| &mut s.title)
                .on_submit(|s: &mut State, tx| s.add(tx));
            let paper = card::blank(card::Blank { counter, title: url, note, error: s.error, url: s.url.text.trim(), category: s.category, strength: 0.6 }, ink);
            let chars = s.url.text.chars().count() + s.title.text.chars().count();
            printer::printer(s.job, paper, printer::feed(chars, s.category.is_some()), keys, ink)
        }
    }
}

fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    let printed = match s.print {
        Print::Ejecting { due, .. } if now >= due => {
            s.land();
            true
        }
        _ => false,
    };
    let shredded = s.step_shred(now, tx);
    printed || shredded
}

fn shredder_view(s: &State, ink: &Ink) -> El {
    let stage = match &s.shred {
        Shred::Idle => shredder::Stage::Idle,
        Shred::Feeding { link, .. } => shredder::Stage::Feeding(link),
        Shred::Shredding { link, .. } => shredder::Stage::Shredding(link),
    };
    // Pinned to the window's corner, over whatever scrolls beneath.
    shredder::shredder(stage, ink).pin(None, None, Some(24.0), Some(20.0))
}

/// The folders, side by side: one per category always, Unsorted while anything is in it.
fn shelf(s: &State, ink: &Ink) -> El {
    let mut folders: Vec<Option<Category>> = Category::ALL.iter().copied().map(Some).collect();
    if s.links.iter().any(|l| l.category.is_none()) {
        folders.push(None);
    }
    let mut shelf = row().key("shelf").w_full().flex_wrap().gap(S3);
    for c in folders {
        let links = s.in_folder(c);
        shelf = shelf.child(folder::folder(c, &links, s.hovered == Some(c), ink, &FOLDER_ACTIONS));
    }
    // The wrapping row sits in a column so its rows pack to the top instead of spreading out.
    col().w_full().pb(S8).child(shelf).scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0)
}

/// The open folder's cards, or everything the search matches.
fn sheet(s: &State, ink: &Ink) -> El {
    let shown = s.visible();
    if shown.is_empty() {
        let message = if !s.search.text.trim().is_empty() { "Nothing matches." } else { "Empty. Print a card while this folder is open." };
        return empty(message, ink);
    }
    let mut sheet = row().key("sheet").w_full().flex_wrap().gap(S6).pb(S8);
    for &(n, link) in &shown {
        sheet = sheet.child(card::card(link, n, s.links.len(), s.hot == Some(link.id), ink, &ACTIONS));
    }
    col().w_full().child(sheet).scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0)
}

const FOLDER_ACTIONS: folder::Actions<State> = folder::Actions {
    open: |s, c| s.open_folder(c),
    hover: |s, c, inside| {
        if inside {
            s.hovered = Some(c);
        } else if s.hovered == Some(c) {
            s.hovered = None;
        }
    },
};

const ACTIONS: Actions<State> = Actions {
    open: open_link,
    remove: |s, id, tx| s.remove(id, tx),
    retag: |s, id| s.retag(id),
    hover: |s, id, inside| {
        if inside {
            s.hot = Some(id);
        } else if s.hot == Some(id) {
            s.hot = None;
        }
    },
};

fn open_link(_: &mut State, url: &str, tx: &mut Tx) {
    if let Err(e) = tx.open_url(url) {
        eprintln!("links: couldn't open {url}: {e:?}");
    }
}

fn empty(message: &'static str, ink: &Ink) -> El {
    col().key("empty").w_full().grow(1.0).min_h(0.0).center().child(text(message).font(type_::BODY).color(ink.dim))
}

fn on_key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    match ev.key {
        Key::Command('q') => tx.quit(),
        Key::Command('w') => tx.close_window(),
        Key::Command('l') | Key::Command('n') => s.url.focused = true,
        Key::Command('f') => s.search.focused = true,
        Key::Escape if s.mode != Mode::Shelf => s.mode = Mode::Shelf,
        _ => return false,
    }
    true
}

fn sample(s: &mut State) {
    let now = store::now();
    let samples = [
        ("https://firecracker-microvm.github.io", "Firecracker microVMs", 300, Some(Category::Vm)),
        ("https://lobste.rs", "", 7200, None),
        ("https://www.inspora.design/posts/stamp-style-cards", "Stamp-style cards", 3 * 86_400, Some(Category::Design)),
        ("https://www.qemu.org/docs/master/", "QEMU documentation", 6 * 86_400, Some(Category::Vm)),
        ("https://www.robinsloan.com/notes/home-cooked-app/", "An app can be a home-cooked meal", 20 * 86_400, Some(Category::Design)),
        ("https://en.wikipedia.org/wiki/Ukiyo-e", "Ukiyo-e", 40 * 86_400, Some(Category::Design)),
        ("https://www.youtube.com/@3blue1brown", "3Blue1Brown", 2 * 86_400, Some(Category::Content)),
        ("https://www.dwarkesh.com/podcast", "Dwarkesh Podcast", 9 * 86_400, Some(Category::Content)),
    ];
    s.persist = false;
    s.links = samples
        .iter()
        .zip(1..)
        .map(|(&(url, title, age, category), id)| Link { id, url: url.into(), title: title.into(), added: now - age, category })
        .collect();
}

fn main() {
    rune::prewarm_gpu();
    rune::run(AppSpec::<State> {
        scenes: &[
            ("sample", sample),
            ("design", |s| {
                sample(s);
                s.open_folder(Some(Category::Design));
            }),
            ("peek", |s| {
                sample(s);
                s.hovered = Some(Some(Category::Design));
            }),
            ("typing", |s| {
                sample(s);
                s.url = TextEdit::new("inkandswitch.com/local-first");
                s.title = TextEdit::new("Local-first software");
                s.category = Some(Category::Design);
            }),
            ("ejecting", |s| {
                sample(s);
                let link = Link { id: 99, url: "https://inkandswitch.com/local-first".into(), title: "Local-first software".into(), added: store::now(), category: Some(Category::Design) };
                s.print = Print::Ejecting { link, due: f64::MAX };
            }),
            ("shredding", |s| {
                sample(s);
                s.open_folder(Some(Category::Design));
                let link = s.links.remove(2);
                s.shred = Shred::Shredding { link, due: f64::MAX };
            }),
            ("hot", |s| {
                sample(s);
                s.open_folder(Some(Category::Design));
                s.hot = Some(3);
            }),
            ("empty", |s| {
                s.persist = false;
                s.links.clear();
            }),
        ],
        on_key: Some(on_key),
        poll: Some(poll),
        min_size: Some((1000.0, 640.0)),
        ..AppSpec::new("Links", (1240.0, 800.0), init, view)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    fn state() -> State {
        let mut s = init_with(Vec::new());
        sample(&mut s);
        s.open_folder(Some(Category::Design));
        s
    }

    #[test]
    fn remove_shreds_the_card_without_opening_the_link() {
        let mut rt = Runtime::new(state(), view, (1000.0, 920.0), Env { system_dark: true, scale: 2.0 });
        rt.frame(0.0);
        let (x, y) = rt.find("card-3/remove").expect("the card's remove button");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert!(rt.take_url_requests().is_empty(), "remove must not also open the link");
        assert!(rt.state.links.iter().all(|l| l.id != 3), "the link is gone from the collection");
        assert!(matches!(rt.state.shred, Shred::Feeding { .. }), "and its card is in the shredder");
    }

    #[test]
    fn clicking_a_card_opens_its_link() {
        let mut rt = Runtime::new(state(), view, (1000.0, 920.0), Env { system_dark: true, scale: 2.0 });
        rt.frame(0.0);
        let (x, y) = rt.find("card-3").expect("the card");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert_eq!(rt.take_url_requests().len(), 1);
    }

    #[test]
    fn clicking_the_tag_retags_without_opening_the_link() {
        let mut rt = Runtime::new(state(), view, (1000.0, 920.0), Env { system_dark: true, scale: 2.0 });
        rt.frame(0.0);
        let (x, y) = rt.find("card-3/tag").expect("the card's tag");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert!(rt.take_url_requests().is_empty());
        assert_eq!(rt.state.links.iter().find(|l| l.id == 3).unwrap().category, Some(Category::Content));
    }
}
