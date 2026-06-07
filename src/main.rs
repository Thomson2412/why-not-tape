use minifb::{Key, Window, WindowOptions};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Camera,
};
use why_not_tape::{decode, encode, Config, Image};

const CAP_W: usize = 32;
const CAP_H: usize = 24;
const SCALE: usize = 10;

fn main() {
    println!("Opening webcam...");
    let index = CameraIndex::Index(0);
    let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut cam = Camera::new(index, req).expect("could not open webcam at /dev/video0");
    cam.open_stream().expect("could not start camera stream");

    println!("Capturing frame...");
    let frame = cam.frame().expect("failed to grab frame");
    let rgb = frame
        .decode_image::<RgbFormat>()
        .expect("failed to decode frame pixels");

    // Extract raw bytes so we can construct an image::RgbImage with our own image version
    let (fw, fh) = (rgb.width(), rgb.height());
    let raw = rgb.into_raw();
    let rgb_img = image::RgbImage::from_raw(fw, fh, raw).expect("buffer size mismatch");

    // Resize to small capture dimensions and convert to grayscale
    let resized = image::DynamicImage::ImageRgb8(rgb_img).resize_exact(
        CAP_W as u32,
        CAP_H as u32,
        image::imageops::FilterType::Lanczos3,
    );
    let input = Image::from_pixels(CAP_W, CAP_H, resized.to_luma8().into_raw());

    // Config: 768 pixels with 5 Hz spacing, 1 Hz freq resolution → zero spectral leakage
    // min=100, max=100+767*5=3935, sample_rate=8000, window=1.0 → freq_resolution=1 Hz
    let config = Config {
        sample_rate: 8000,
        min_freq: 100.0,
        max_freq: 3935.0,
        window: 1.0,
    };

    let n = CAP_W * CAP_H;
    println!(
        "Encoding {n} pixels → {} audio samples...",
        config.total_samples()
    );
    let samples = encode(&input, &config);

    println!("Decoding audio → image...");
    let output = decode(&samples, &config, CAP_W, CAP_H);

    println!("Showing result — press Esc to close.");

    // Layout: [original] gap [round-trip]
    let gap = SCALE;
    let win_w = CAP_W * SCALE * 2 + gap;
    let win_h = CAP_H * SCALE;
    let mut buf = vec![0u32; win_w * win_h];

    blit(&mut buf, win_w, &input, 0);
    blit(&mut buf, win_w, &output, CAP_W * SCALE + gap);

    let mut win = Window::new(
        "why-not-tape  |  original (left)  ·  round-trip (right)  |  Esc to quit",
        win_w,
        win_h,
        WindowOptions::default(),
    )
    .expect("failed to create window");

    while win.is_open() && !win.is_key_down(Key::Escape) {
        win.update_with_buffer(&buf, win_w, win_h).unwrap();
    }
}

fn blit(buf: &mut [u32], buf_width: usize, img: &Image, x_off: usize) {
    for row in 0..img.height {
        for col in 0..img.width {
            let v = img.pixel(col, row) as u32;
            let color = (v << 16) | (v << 8) | v;
            for dy in 0..SCALE {
                for dx in 0..SCALE {
                    buf[(row * SCALE + dy) * buf_width + x_off + col * SCALE + dx] = color;
                }
            }
        }
    }
}
