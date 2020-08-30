# What is this

A silly personal site built with [Yew](https://github.com/yewstack/yew).

Also uses [Normalize CSS](https://github.com/necolas/normalize.css) and [Holiday CSS](https://holidaycss.js.org/).

# Building

Tested with rust 1.46.0, see [Rustup](https://rustup.rs/) for installation.

First, install wasm-pack:

```
cargo install wasm-pack
```

Then build:

```
./build.sh
```

To serve the static site for testing:

```
python3 -m http.server --directory static
```

# Can I put my own info in it?

Sure. `cp sites.demo.json assets/site.json` and edit `assets/site.json` to your linking, then rebuild.

# License

GPLv3, see [COPYING](./COPYING).
