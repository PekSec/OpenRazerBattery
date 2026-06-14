use crate::error::AppError;

pub const RAZER_REPORT_LEN: usize = 90;
pub const WINDOWS_FEATURE_REPORT_LEN: usize = 91;
pub const REPORT_ID: u8 = 0x00;

pub const CMD_CLASS_POWER: u8 = 0x07;
pub const CMD_GET_BATTERY: u8 = 0x80;
pub const CMD_GET_CHARGING: u8 = 0x84;

pub const INDEX_TRANSACTION_ID: usize = 1;
pub const INDEX_DATA_SIZE: usize = 5;
pub const INDEX_COMMAND_CLASS: usize = 6;
pub const INDEX_COMMAND_ID: usize = 7;
pub const INDEX_ARGUMENTS_START: usize = 8;
pub const INDEX_CHECKSUM: usize = 88;

const CHECKSUM_START: usize = 2;
const CHECKSUM_END_EXCLUSIVE: usize = 88;

pub fn build_get_battery_report(transaction_id: u8) -> [u8; RAZER_REPORT_LEN] {
    build_power_report(transaction_id, CMD_GET_BATTERY)
}

pub fn build_get_charging_report(transaction_id: u8) -> [u8; RAZER_REPORT_LEN] {
    build_power_report(transaction_id, CMD_GET_CHARGING)
}

pub fn build_power_report(transaction_id: u8, command_id: u8) -> [u8; RAZER_REPORT_LEN] {
    let mut report = [0; RAZER_REPORT_LEN];
    report[INDEX_TRANSACTION_ID] = transaction_id;
    report[INDEX_DATA_SIZE] = 0x02;
    report[INDEX_COMMAND_CLASS] = CMD_CLASS_POWER;
    report[INDEX_COMMAND_ID] = command_id;
    report[INDEX_CHECKSUM] = checksum_for_report(&report);
    report
}

pub fn to_windows_feature_report(
    report: &[u8; RAZER_REPORT_LEN],
) -> [u8; WINDOWS_FEATURE_REPORT_LEN] {
    let mut buffer = [0; WINDOWS_FEATURE_REPORT_LEN];
    buffer[0] = REPORT_ID;
    buffer[1..].copy_from_slice(report);
    buffer
}

pub fn calculate_checksum(report: &[u8]) -> Result<u8, AppError> {
    if report.len() != RAZER_REPORT_LEN {
        return Err(AppError::InvalidReportLength {
            expected: RAZER_REPORT_LEN,
            actual: report.len(),
        });
    }

    Ok(report[CHECKSUM_START..CHECKSUM_END_EXCLUSIVE]
        .iter()
        .fold(0, |checksum, byte| checksum ^ byte))
}

pub fn checksum_for_report(report: &[u8; RAZER_REPORT_LEN]) -> u8 {
    report[CHECKSUM_START..CHECKSUM_END_EXCLUSIVE]
        .iter()
        .fold(0, |checksum, byte| checksum ^ byte)
}

pub fn set_checksum(report: &mut [u8]) -> Result<(), AppError> {
    let checksum = calculate_checksum(report)?;
    report[INDEX_CHECKSUM] = checksum;
    Ok(())
}

pub fn validate_response(
    report: &[u8],
    expected_command_class: u8,
    expected_command_id: u8,
) -> Result<(), AppError> {
    let checksum = calculate_checksum(report)?;

    if report[INDEX_CHECKSUM] != checksum {
        return Err(AppError::InvalidChecksum);
    }

    if report[INDEX_COMMAND_CLASS] != expected_command_class {
        return Err(AppError::UnexpectedResponse);
    }

    if report[INDEX_COMMAND_ID] != expected_command_id {
        return Err(AppError::UnexpectedResponse);
    }

    Ok(())
}
