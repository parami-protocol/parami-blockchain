use crate::images;
use core::f32::consts::PI;
#[cfg(not(feature = "std"))]
use num_traits::Float;
use sp_std::prelude::*;

/// Parse downloaded image file
pub fn parse(data: &[u8]) -> Option<Vec<u8>> {
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

    const JPG: &[u8] = include_bytes!("../artifacts/did.jpg");
    const PNG: &[u8] = include_bytes!("../artifacts/did.png");
    // 32ac799d35de72a2ae57a46ca975319fbbb125a9
    const DID: [u8; 20] = [
        0x32, 0xac, 0x79, 0x9d, 0x35, 0xde, 0x72, 0xa2, 0xae, 0x57, 0xa4, 0x6c, 0xa9, 0x75, 0x31,
        0x9f, 0xbb, 0xb1, 0x25, 0xa9,
    ];

    #[test]
    fn should_decode() {
        assert_eq!(parse(JPG), Some(DID.to_vec()));
        assert_eq!(parse(PNG), Some(DID.to_vec()));
    }
}
