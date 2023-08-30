use image::{DynamicImage, GenericImageView, Pixel};
use log::debug;

pub fn image_file_is_mictest(path: &str) -> anyhow::Result<bool> {
    let img = image::open(path)?;
    Ok(image_is_mictest(img))
}

pub fn image_is_mictest(img: DynamicImage) -> bool {
    let mut non_black_white_pixels = 0;

    img.pixels().all(|(_x, _y, rgba)| {
        let c = rgba.channels();
        let r = c[0];
        let g = c[1];
        let b = c[2];
        let is_black = r < 20 && g < 20 && b < 20;
        let is_white = r > 220 && g > 220 && b > 220;
        let is_grey = r == g && g == b;
        if !is_black && !is_white && !is_grey {
            non_black_white_pixels += 1;
            if non_black_white_pixels > img.width() * img.height() / 100 {
                debug!("rejecting image because it has too many non-black/white pixels");
                return false;
            }
        }

        return true;
    }) && img.pixels().any(|(_x, _y, rgba)| {
        let c = rgba.channels();
        let r = c[0];
        let g = c[1];
        let b = c[2];

        let is_black = r < 20 && g < 20 && b < 20;
        !is_black
    })
}
