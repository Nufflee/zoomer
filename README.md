# zoomer

`zoomer` is a WIP and currently Windows only application which will allow you to zoom in and out of your screen and point things out. It is mainly intended for use by the author during his streams and the like, but everyone is welcome.

This applications interacts with Win32 API and OpenGL directly, without using any window management or OpenGL loader libraries - the goal is to use as few dependencies as possible for simplicity.

## > running
**IMPORTANT**: The `x86_64-pc-windows-gnu` Rust target needs to be used because the `stb` library uses [`rust-bindgen`](https://github.com/rust-lang/rust-bindgen) which requires `clang`.

```sh
$ cargo run --target x86_64-pc-windows-gnu
```