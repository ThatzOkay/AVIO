//! AOAP handshake — switch a stock Android phone into accessory mode.
//! Spec: <https://source.android.com/docs/core/interaction/accessories/aoa>

use std::time::Duration;

use nusb::transfer::{ControlIn, ControlOut, ControlType, Recipient, TransferError};
use nusb::{Device, DeviceInfo, Interface};

use super::constants::{
    ACCESSORY_PIDS, AOAP_DESCRIPTION, AOAP_MANUFACTURER, AOAP_MODEL, AOAP_SERIAL, AOAP_URI,
    AOAP_VERSION, GOOGLE_VID, REQ_GET_PROTOCOL, REQ_SEND_STRING, REQ_START, STRING_DESCRIPTION,
    STRING_MANUFACTURER, STRING_MODEL, STRING_SERIAL, STRING_URI, STRING_VERSION,
};

const TRANSFER_TIMEOUT: Duration = Duration::from_millis(2_000);

#[derive(Debug)]
pub enum HandshakeError {
    NoClaimableInterface,
    Transfer(TransferError),
    ProtocolTooOld(u16),
    ShortProtocolReply,
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandshakeError::NoClaimableInterface => {
                write!(f, "AOAP: no claimable interface for control transfers")
            }
            HandshakeError::Transfer(e) => write!(f, "AOAP control transfer failed: {e}"),
            HandshakeError::ProtocolTooOld(v) => {
                write!(f, "AOAP protocol version {v} not supported by device")
            }
            HandshakeError::ShortProtocolReply => write!(f, "AOAP getProtocol returned no data"),
        }
    }
}

impl std::error::Error for HandshakeError {}

impl From<TransferError> for HandshakeError {
    fn from(e: TransferError) -> Self {
        HandshakeError::Transfer(e)
    }
}

pub fn is_accessory_mode(info: &DeviceInfo) -> bool {
    println!("Checking if device is in accessory mode: {:?}", info);
    info.vendor_id() == GOOGLE_VID && ACCESSORY_PIDS.contains(&info.product_id())
}

// nusb routes a device-recipient control transfer through any claimed interface, and errors if
// none is claimed. The claim need not be interface 0, which on macOS the kernel driver (MTP/PTP)
// may hold — claim the first interface that is actually claimable (e.g. the vendor one).
async fn claim_any_interface(device: &Device) -> Result<Interface, HandshakeError> {
    let interface_numbers: Vec<u8> = device
        .active_configuration()
        .map(|config| config.interfaces().map(|i| i.interface_number()).collect())
        .unwrap_or_default();

    for number in interface_numbers {
        if let Ok(interface) = device.claim_interface(number).await {
            return Ok(interface);
        }
    }

    Err(HandshakeError::NoClaimableInterface)
}

async fn get_protocol(interface: &Interface) -> Result<u16, HandshakeError> {
    let data = interface
        .control_in(
            ControlIn {
                control_type: ControlType::Vendor,
                recipient: Recipient::Device,
                request: REQ_GET_PROTOCOL,
                value: 0,
                index: 0,
                length: 2,
            },
            TRANSFER_TIMEOUT,
        )
        .await?;

    if data.len() < 2 {
        return Err(HandshakeError::ShortProtocolReply);
    }
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

async fn send_string(interface: &Interface, index: u16, value: &str) -> Result<(), HandshakeError> {
    let mut data = value.as_bytes().to_vec();
    data.push(0);
    interface
        .control_out(
            ControlOut {
                control_type: ControlType::Vendor,
                recipient: Recipient::Device,
                request: REQ_SEND_STRING,
                value: 0,
                index,
                data: &data,
            },
            TRANSFER_TIMEOUT,
        )
        .await?;
    Ok(())
}

async fn start_accessory(interface: &Interface) -> Result<(), HandshakeError> {
    interface
        .control_out(
            ControlOut {
                control_type: ControlType::Vendor,
                recipient: Recipient::Device,
                request: REQ_START,
                value: 0,
                index: 0,
                data: &[],
            },
            TRANSFER_TIMEOUT,
        )
        .await?;
    Ok(())
}

/// Runs the AOAP handshake against a phone that isn't in accessory mode yet. On success the
/// phone disconnects and re-enumerates under one of `ACCESSORY_PIDS` — the caller is responsible
/// for waiting for that reattach (see `transport::usb_aoap_bridge`).
pub async fn run_aoap_handshake(device: &Device) -> Result<(), HandshakeError> {
    let interface = claim_any_interface(device).await?;

    let proto = get_protocol(&interface).await?;
    if proto < 1 {
        return Err(HandshakeError::ProtocolTooOld(proto));
    }

    send_string(&interface, STRING_MANUFACTURER, AOAP_MANUFACTURER).await?;
    send_string(&interface, STRING_MODEL, AOAP_MODEL).await?;
    send_string(&interface, STRING_DESCRIPTION, AOAP_DESCRIPTION).await?;
    send_string(&interface, STRING_VERSION, AOAP_VERSION).await?;
    send_string(&interface, STRING_URI, AOAP_URI).await?;
    send_string(&interface, STRING_SERIAL, AOAP_SERIAL).await?;

    start_accessory(&interface).await?;
    Ok(())
}
