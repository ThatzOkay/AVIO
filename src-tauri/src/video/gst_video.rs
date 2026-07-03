
pub enum GstVideoCodec {
    H264,
    H265,
    VP9,
    AV1,
}

fn hex_to_rgb255(hex: Option<&str>) -> (u8, u8, u8) {
    let s = hex.unwrap_or("").trim();
    let s = s.strip_prefix('#').unwrap_or(s);

    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return (0, 0, 0);
    }

    let n = u32::from_str_radix(s, 16).unwrap_or(0);
    (((n >> 16) & 0xff) as u8, ((n >> 8) & 0xff) as u8, (n & 0xff) as u8)
}

