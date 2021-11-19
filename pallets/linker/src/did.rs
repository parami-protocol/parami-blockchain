use crate::images;
use core::f32::consts::PI;
#[cfg(not(feature = "std"))]
use num_traits::Float;
use sp_std::prelude::*;

/// Parse downloaded image file
pub fn parse(data: Vec<u8>) -> Option<Vec<u8>> {
    let image = match images::decode_jpeg(data) {
        Some(image) => image,
        None => return None,
    };

    let mut binary = Vec::<u8>::new();

    const THRESHOLD: u8 = 220;
    const STEP: f32 = 2f32 * PI / 3f32 / 180f32;

    let r = (342f32 * image.width() as f32 / 640f32 * 10f32).round() / 10f32;
    let mut angle = PI / 6f32 + STEP / 2f32;

    let mut byte = 0u8;
    for j in 0..180 {
        if j % 9 == 0 {
            byte = 0;
        }
        if j % 9 == 8 {
            binary.push(byte);
        }

        byte = byte << 1;

        let x = image.width() as f32 / 2f32 + r * angle.cos();
        let y = image.height() as f32 / 2f32 + r * angle.sin();
        let pixel = image.pixel(x.floor() as u32, y.floor() as u32);

        if pixel >= THRESHOLD {
            byte |= 1;
        }

        angle += STEP;
        if (j + 1) % 45 == 0 {
            angle += 2f32 * PI / 6f32;
        }
    }

    Some(binary)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::num::ParseIntError;

    const JPG: &[u8] = include_bytes!("../artifacts/did.jpg");
    const PNG: &[u8] = include_bytes!("../artifacts/did.png");
    const DID: &str = "32ac799d35de72a2ae57a46ca975319fbbb125a9";

    fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect()
    }

    #[test]
    fn should_decode() {
        assert_eq!(parse(JPG.to_vec()), decode_hex(DID).ok());
        assert_eq!(parse(PNG.to_vec()), decode_hex(DID).ok());
    }
}
