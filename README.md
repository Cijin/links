# Links

A small Mac app for saving links to interesting websites, built with [Rune](https://github.com/anomalyco/rune).
Links sit in folders by category. Each is a dark card with a glowing keypad whose lit keys come from the site's name. The look follows "Retro CRT" on Inspora, and the folders follow "Stamp Folder animation".

## Run

```sh
cargo run --release            # open the window
./scripts/bundle.sh            # build dist/Links.app
./scripts/bundle.sh --install  # and copy it to ~/Applications
```

Rust comes from Homebrew's `rustup`, which is keg-only. Put it on your PATH:
`export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.

## Use

- New cards come out of the little printer. Each character you type feeds the card up, and backspace rolls it back. Paste a link (⌘V anywhere drops it into the form), add a title if you like, and press Return or Save: the card rolls out, then flies into its folder.
- Pick **VM**, **Design** or **Content** next to the title field before saving. The choice stays picked for the next link, and clicking it again clears it.
- Links live in folders: **VM**, **Design**, **Content**, and **Unsorted** (shown while anything is uncategorized). Each has a pixel icon. Hover a folder to fan out its newest cards; click it to open it. "← Folders" or Esc goes back. A link saved while a folder is open goes into that folder.
- Typing in the search field searches every folder from the shelf, or just the open one.
- Click the tag on a card to change its category (none → VM → Design → Content → none).
- Click a card to open it in your browser. "Remove" deletes it: the card flies into the shredder in the bottom-right corner and comes out as strips.
- ⌘F searches, ⌘L / ⌘N goes back to the link field, ⌘Q quits.

Links are stored in `~/Library/Application Support/Links/links.json`. Set `LINKS_FILE` to use another file.

## Check it headlessly

```sh
cargo run --release -- --snap links.png --scene sample   # render the sample sheet to a PNG
cargo run --release -- --snap peek.png --scene peek      # the shelf, Design folder hovered
cargo run --release -- --snap design.png --scene design  # the Design folder open
cargo run --release -- --snap hot.png --scene hot        # the same, one card hovered
cargo run --release -- --snap typing.png --scene typing  # a card half fed out of the printer
cargo run --release -- --snap out.png --scene ejecting   # a printed card, all the way out
cargo run --release -- --snap shred.png --scene shredding # a card going through the shredder
cargo run --release -- --tree tree.json --scene sample   # dump the element tree
cargo test
```

## Layout

- `src/main.rs`: state, the form, search, the shelf of folders and the grid of cards.
- `src/printer.rs`: the printer and the paper feed.
- `src/shredder.rs`: the shredder, its mouth and the strips.
- `src/folder.rs`: the folder with cards tucked in.
- `src/card.rs`: the palette, the card, the mini card, the keypad pictures and the category icons.
- `src/store.rs`: loading, saving and cleaning up URLs.

Rune is pinned to a `dev` rev in `Cargo.toml`. Bump the rev to upgrade.
