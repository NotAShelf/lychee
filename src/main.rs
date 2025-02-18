use image::{self, GenericImageView};
use minifb::{Key, Window, WindowOptions};
use std::path::Path;
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Please provide a path to the image file.");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let img = image::open(Path::new(file_path))?;
    let (img_width, img_height) = img.dimensions();

    let same_size = args.contains(&"--same-size".to_string());

    let img_rgba = img.to_rgba8();

    let mut buffer: Vec<u32> = vec![0; (img_width * img_height) as usize];
    for (rgba, argb) in img_rgba.chunks(4).zip(buffer.iter_mut()) {
        let r = rgba[0] as u32;
        let g = rgba[1] as u32;
        let b = rgba[2] as u32;
        let a = rgba[3] as u32;
        *argb = (a << 24) | (r << 16) | (g << 8) | b;
    }

    let (window_width, window_height) = if same_size {
        (img_width as usize, img_height as usize)
    } else {
        (1920, 1080)
    };

    let mut window = Window::new(
        "lychee",
        window_width,
        window_height,
        WindowOptions {
            resize: true,
            borderless: true,
            ..WindowOptions::default()
        },
    )?;

    window.set_target_fps(60);

    let mut resize_buffer: Vec<u32> = vec![0; window_width * window_height];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if same_size {
            window.update_with_buffer(&buffer, img_width as usize, img_height as usize)?;
        } else {
            resize_buffer.fill(0);

            let x_offset = (window_width - img_width as usize) / 2;
            let y_offset = (window_height - img_height as usize) / 2;

            for (i, &pixel) in buffer.iter().enumerate() {
                let x = i % img_width as usize;
                let y = i / img_width as usize;

                if x + x_offset < window_width && y + y_offset < window_height {
                    let index = (y + y_offset) * window_width + (x + x_offset);
                    resize_buffer[index] = pixel;
                }
            }

            window.update_with_buffer(&resize_buffer, window_width, window_height)?;
        }
    }

    Ok(())
}
