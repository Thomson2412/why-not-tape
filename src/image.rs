/// Grayscale image. Pixels are stored row-major; 0 = black, 255 = white.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pixels: Vec<u8>,
}

impl Image {
    /// Creates a new all-black image.
    pub fn new(width: usize, height: usize) -> Self {
        Image {
            width,
            height,
            pixels: vec![0u8; width * height],
        }
    }

    /// Constructs an image from an existing row-major pixel buffer.
    ///
    /// # Panics
    /// Panics if `pixels.len() != width * height`.
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<u8>) -> Self {
        if width * height != pixels.len() {
            panic!("Image size does not match amount of pixels")
        }
        Image {
            width,
            height,
            pixels,
        }
    }

    /// Returns the total amount of a pixels..
    pub fn pixel_amount(&self) -> usize {
        self.width * self.height
    }

    /// Returns the intensity of a pixel (0–255).
    ///
    /// # Panics
    /// Panics if `col >= width` or `row >= height`.
    pub fn pixel(&self, col: usize, row: usize) -> u8 {
        check_col_row_bounds(self.width, self.height, col, row);
        self.pixels[row * self.width + col]
    }

    /// Returns the intensity of a pixel (0–255).
    ///
    /// # Panics
    /// Panics if `index >= pixel_amount`.
    pub fn pixel_by_index(&self, index: usize) -> u8 {
        if index >= self.pixel_amount() {
            panic!("Index is out of bounds")
        }
        self.pixels[index]
    }

    /// Sets the intensity of a pixel (0–255).
    ///
    /// # Panics
    /// Panics if `col >= width` or `row >= height`.
    pub fn set_pixel(&mut self, col: usize, row: usize, value: u8) {
        check_col_row_bounds(self.width, self.height, col, row);
        self.pixels[row * self.width + col] = value;
    }

    /// Sets the intensity of a pixel (0–255).
    ///
    /// # Panics
    /// Panics if `col >= width` or `row >= height`.
    pub fn set_pixel_by_index(&mut self, index: usize, value: u8) {
        if index >= self.pixel_amount() {
            panic!("Index is out of bounds")
        }
        self.pixels[index] = value;
    }

    /// Saves the image to `path` as a grayscale PNG (or any format the `image`
    /// crate infers from the file extension).
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), image::ImageError> {
        let gray =
            image::GrayImage::from_raw(self.width as u32, self.height as u32, self.pixels.clone())
                .expect("pixel buffer size must equal width * height");
        gray.save(path)
    }
}

fn check_col_row_bounds(width: usize, height: usize, col: usize, row: usize) {
    if col >= width || row >= height {
        panic!("Row or col out of bounds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_amount_is_correct() {
        let width = 4;
        let height = 3;
        let img = Image::new(width, height);
        assert_eq!(img.pixel_amount(), width * height);
    }

    #[test]
    fn new_image_is_all_black() {
        let img = Image::new(4, 3);
        for row in 0..3 {
            for col in 0..4 {
                assert_eq!(img.pixel(col, row), 0);
            }
        }
    }

    #[test]
    fn new_image_has_correct_dimensions() {
        let img = Image::new(7, 5);
        assert_eq!(img.width, 7);
        assert_eq!(img.height, 5);
    }

    #[test]
    fn set_and_get_pixel_roundtrips() {
        let mut img = Image::new(4, 3);
        img.set_pixel(2, 1, 128);
        assert_eq!(img.pixel(2, 1), 128);
    }

    #[test]
    fn set_and_get_pixel_roundtrips_by_index() {
        let mut img = Image::new(4, 3);
        let index = 4;
        img.set_pixel_by_index(index, 128);
        assert_eq!(img.pixel_by_index(index), 128);
    }

    #[test]
    fn set_pixel_does_not_affect_neighbours() {
        let mut img = Image::new(3, 3);
        img.set_pixel(1, 1, 255);
        assert_eq!(img.pixel(0, 0), 0);
        assert_eq!(img.pixel(2, 0), 0);
        assert_eq!(img.pixel(0, 2), 0);
        assert_eq!(img.pixel(2, 2), 0);
    }

    #[test]
    fn from_pixels_reads_row_major_order() {
        //  [10, 20, 30]
        //  [40, 50, 60]
        let img = Image::from_pixels(3, 2, vec![10, 20, 30, 40, 50, 60]);
        assert_eq!(img.pixel(0, 0), 10);
        assert_eq!(img.pixel(1, 0), 20);
        assert_eq!(img.pixel(2, 0), 30);
        assert_eq!(img.pixel(0, 1), 40);
        assert_eq!(img.pixel(1, 1), 50);
        assert_eq!(img.pixel(2, 1), 60);
    }

    #[test]
    #[should_panic]
    fn from_pixels_panics_on_wrong_buffer_size() {
        Image::from_pixels(3, 2, vec![0; 5]); // should be 6
    }

    #[test]
    #[should_panic]
    fn pixel_by_index_panics_on_wrong_index_out_of_bounds() {
        let img = Image::new(3, 3);
        img.pixel_by_index(9); // should be 8
    }

    #[test]
    #[should_panic]
    fn pixel_panics_on_out_of_bounds_col() {
        Image::new(3, 3).pixel(3, 0);
    }

    #[test]
    #[should_panic]
    fn pixel_panics_on_out_of_bounds_row() {
        Image::new(3, 3).pixel(0, 3);
    }

    #[test]
    fn save_image_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.png");
        let img = Image::new(4, 3);
        img.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn save_image_roundtrips_pixels() {
        let pixels: Vec<u8> = (0u8..12).collect();
        let img = Image::from_pixels(4, 3, pixels.clone());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.png");
        img.save(&path).unwrap();

        let loaded = image::open(&path).unwrap().into_luma8();
        let reloaded = Image::from_pixels(4, 3, loaded.into_raw());
        assert_eq!(img, reloaded);
    }

    #[test]
    fn save_image_roundtrips_single_pixel() {
        let img = Image::from_pixels(1, 1, vec![200]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("single.png");
        img.save(&path).unwrap();

        let loaded = image::open(&path).unwrap().into_luma8();
        assert_eq!(loaded.get_pixel(0, 0).0[0], 200);
    }

    #[test]
    fn save_image_returns_error_on_bad_path() {
        let img = Image::new(2, 2);
        let result = img.save("/nonexistent_dir/out.png");
        assert!(result.is_err());
    }
}
