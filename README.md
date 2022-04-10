# zoomer

`zoomer` is a WIP and currently Windows only application which will allow you to zoom in and out of your screen and point things out. It is mainly intended for use by the author during his streams and the like, but everyone is welcome.

This applications interacts with Win32 API and OpenGL directly, without using any window management or OpenGL loader libraries - the goal is to use as few dependencies as possible for simplicity.

## > running

Note: Nightly Rust is currently required due to the usage of `backtrace` feature.

```sh
$ cargo run --release
```

## > usage
| Input                          | Description                        |
| ------------------------------ | ---------------------------------- |
| <kbd>Alt</kbd> + <kbd>A</kbd>  | Show the zoomer window             |
| <kbd>Esc</kbd>                 | Hide the zoomer window             |
| Drag with Left Mouse button    | Pan around                         |
| Scroll Wheel                   | Zoom in and out                    |
| <kbd>C</kbd>                   | Toggle the highlighter             |
| <kbd>Ctrl</kbd> + Scroll Wheel | Change the size of the highlighter |
| <kbd>F2</kbd>                  | Toggle debug UI                    |