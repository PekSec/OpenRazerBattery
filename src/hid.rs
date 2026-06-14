use crate::{
    device::{RAZER_VID, RazerHidCandidate},
    error::AppError,
};

pub fn enumerate_razer_hid_candidates() -> Result<Vec<RazerHidCandidate>, AppError> {
    let _ = RAZER_VID;
    Ok(Vec::new())
}
