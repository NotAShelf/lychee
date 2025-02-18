# Lychee

[Image crate]: https://crates.io/crates/image

Simple, opinionated and Wayland-native image viewer using the [Image crate] .
Built in response to `imv`'s high memory usage on my system, and does not
implement an IPC as I consider it _YAGNI_[^1]

Still a work in progress, I would like to reach partial feature parity with
`imv`.

## Usage

```bash
lychee path/to/image.png # or another extension.
```

You may use the `--same-size` flag to create a window that is the _same size as
the image that is opening_. Success of this flag depends on your desktop
environment.

## Supported Image Formats

All formats supported by the Image crate will be supported by Lychee. For edge
cases, please open an issue. For the time being, this should include: AVIF, BMP,
DDS, Farbfeld, GIF, HDR, ICO, JPEG, EXR, PNG, PNM, QOI, TGA, TIFF and WebP.

## License

Lychee is available under the [Mozilla Public License Version 2.0](./LICENSE)

[^1]. ["You Aren't Gonna Need It"](https://martinfowler.com/bliki/Yagni.html)
