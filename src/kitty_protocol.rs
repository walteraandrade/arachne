use base64::{Engine, engine::general_purpose::STANDARD};

const CHUNK_SIZE: usize = 4096;

pub fn encode_kitty_image(image_id: u32, png_bytes: &[u8], cols: u16, rows: u16) -> String {
    let b64 = STANDARD.encode(png_bytes);
    let chunks: Vec<&str> = b64
        .as_bytes()
        .chunks(CHUNK_SIZE)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();

    let mut out = String::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == chunks.len() - 1;
        let more = if is_last { 0 } else { 1 };

        out.push_str("\x1b_G");
        if is_first {
            out.push_str(&format!(
                "a=T,f=100,q=2,i={image_id},c={cols},r={rows},m={more};"
            ));
        } else {
            out.push_str(&format!("m={more};"));
        }
        out.push_str(chunk);
        out.push_str("\x1b\\");
    }
    out
}

pub fn delete_kitty_images_at_cursor() -> String {
    "\x1b_Ga=d,d=C,q=2;\x1b\\".to_string()
}

pub fn delete_all_kitty_images() -> String {
    "\x1b_Ga=d,d=a,q=2;\x1b\\".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_produces_valid_apc_sequence() {
        let png = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic header bytes
        let encoded = encode_kitty_image(1, &png, 4, 1);
        assert!(encoded.starts_with("\x1b_G"));
        assert!(encoded.contains("a=T,f=100,q=2,i=1,c=4,r=1"));
        assert!(encoded.ends_with("\x1b\\"));
    }

    #[test]
    fn encode_chunks_large_payload() {
        let png = vec![0xAA; 8000]; // large enough to require 2+ chunks
        let encoded = encode_kitty_image(2, &png, 8, 1);
        let apc_count = encoded.matches("\x1b_G").count();
        assert!(apc_count >= 2, "should chunk into multiple APC sequences");
        // first chunk should have m=1 (more), last should have m=0
        assert!(encoded.contains("m=1;"));
        assert!(encoded.contains("m=0;"));
    }

    #[test]
    fn delete_produces_valid_sequence() {
        let del = delete_kitty_images_at_cursor();
        assert!(del.starts_with("\x1b_G"));
        assert!(del.contains("a=d,d=C"));
    }
}
