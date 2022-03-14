# zoomer

`zoomer` is a WIP and currently Windows only application which will allow you to zoom in and out of your screen and point things out. It is mainly intended for use by the author during his streams and the like, but everyone is welcome.

This applications interacts with Win32 API and OpenGL directly, without using any window management or OpenGL loader libraries - the goal is to use as few dependencies as possible for simplicity.

## > running

Note: Nightly Rust is currently required.

```sh
$ cargo run --release
```