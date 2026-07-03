pub const CARLINKIT_VID: u16 = 0x1314;
pub const CARLINKIT_PIDS: [u16; 2] = [0x1520, 0x1521];

pub fn is_carlinkit_dongle(vid: Option<u16>, pid: Option<u16>) -> bool {
    match (vid, pid) {
        (Some(vid), Some(pid)) => vid == CARLINKIT_VID && CARLINKIT_PIDS.contains(&pid),
        _ => false,
    }
}
