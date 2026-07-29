//! `aa-status` Tauri event payload — shared by every AA driver (currently just `wired_driver`,
//! eventually a wireless one too) so they all emit the same shape to the frontend.

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AAStatus {
    pub status: String,
    pub device_name: Option<String>,
}
