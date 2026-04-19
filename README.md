# Lychee

[Image crate]: https://crates.io/crates/image
[Iced]: https://iced.rs

Simple, opinionated image viewer using [Iced] and the [Image crate]. Built to
replace `imv` on my system, particularly in response to its high memory usage
and lack of UI polish. Unlike imv, Lychee does not implement an IPC as I consider
it _YAGNI_[^1] but it improves a more powerful UI and a stronger focus on user
experience.

## Usage

Using lychee is easy. Give it something to display. That's it.

```bash
# View a single image. Wow, much png.
$ lychee path/to/image.png

# View all images in directory
$ lychee path/to/directory/

# View specific images
$ lychee image1.png image2.png
```

This can be an image, multiple images, or a path containing images.

## Keybindings

### Navigation

| Key               | Action            |
| ----------------- | ----------------- |
| `→` or `l` or `n` | Next image        |
| `←` or `h` or `p` | Previous image    |
| `g`               | Go to first image |
| `G`               | Go to last image  |

### Zoom

| Key               | Action            |
| ----------------- | ----------------- |
| `↑` or `i` or `+` | Zoom in           |
| `↓` or `o` or `-` | Zoom out          |
| `0`               | Fit to window     |
| `1`               | Actual size (1:1) |
| `r`               | Reset zoom        |

### Slideshow

| Key     | Action                         |
| ------- | ------------------------------ |
| `Space` | Toggle slideshow               |
| `t`     | Increase slideshow delay by 1s |
| `T`     | Decrease slideshow delay by 1s |

### Window

| Key          | Action            |
| ------------ | ----------------- |
| `f`          | Toggle fullscreen |
| `q` or `Esc` | Quit              |

### Mouse

- **Scroll**: Zoom in/out
- **Click + Drag**: Pan image
- **Window close button**: Quit

## Supported Image Formats

Lychee supports a wide array of formats. Primarily, those supported by the
[Image crate] will be supported verbatim in Lychee. At the time of writing,
includes: AVIF, BMP, DDS, Farbfeld, GIF, HDR, ICO, JPEG, EXR, PNG, PNM, QOI,
TGA, TIFF and WebP.

More formats may be supported in the future on demand.

## License

This project is made available under Mozilla Public License (MPL) version 2.0.
See [LICENSE](LICENSE) for more details on the exact conditions. An online copy
is provided [here](https://www.mozilla.org/en-US/MPL/2.0/).

[^1]: ["You Aren't Gonna Need It"](https://martinfowler.com/bliki/Yagni.html)
