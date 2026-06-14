use crate::error::AppError;

pub const RAZER_REPORT_LEN: usize = 90;
pub const WINDOWS_FEATURE_REPORT_LEN: usize = 91;
pub const REPORT_ID: u8 = 0x00;

pub const CMD_CLASS_POWER: u8 = 0x07;
pub const CMD_GET_BATTERY: u8 = 0x80;
pub const CMD_GET_CHARGING: u8 = 0x84;
pub const POWER_REPORT_DATA_SIZE: u8 = 0x02;
pub const PROTOCOL_TYPE: u8 = 0x00;

pub const STATUS_NEW_COMMAND: u8 = 0x00;
pub const STATUS_BUSY: u8 = 0x01;
pub const STATUS_SUCCESS: u8 = 0x02;
pub const STATUS_FAILURE: u8 = 0x03;
pub const STATUS_TIMEOUT: u8 = 0x04;
pub const STATUS_NOT_SUPPORTED: u8 = 0x05;

pub const INDEX_STATUS: usize = 0;
pub const INDEX_TRANSACTION_ID: usize = 1;
pub const INDEX_REMAINING_PACKETS_MSB: usize = 2;
pub const INDEX_REMAINING_PACKETS_LSB: usize = 3;
pub const INDEX_PROTOCOL_TYPE: usize = 4;
pub const INDEX_DATA_SIZE: usize = 5;
pub const INDEX_COMMAND_CLASS: usize = 6;
pub const INDEX_COMMAND_ID: usize = 7;
pub const INDEX_ARGUMENTS_START: usize = 8;
pub const INDEX_ARGUMENT_0: usize = INDEX_ARGUMENTS_START;
pub const INDEX_ARGUMENT_1: usize = INDEX_ARGUMENTS_START + 1;
pub const INDEX_POWER_VALUE: usize = INDEX_ARGUMENT_1;
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
    report[INDEX_STATUS] = STATUS_NEW_COMMAND;
    report[INDEX_TRANSACTION_ID] = transaction_id;
    report[INDEX_PROTOCOL_TYPE] = PROTOCOL_TYPE;
    report[INDEX_DATA_SIZE] = POWER_REPORT_DATA_SIZE;
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

pub fn from_windows_feature_report(
    feature_report: &[u8; WINDOWS_FEATURE_REPORT_LEN],
) -> Result<[u8; RAZER_REPORT_LEN], AppError> {
    if feature_report[0] != REPORT_ID {
        return Err(AppError::UnexpectedResponse);
    }

    let mut report = [0; RAZER_REPORT_LEN];
    report.copy_from_slice(&feature_report[1..]);
    Ok(report)
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

    match report[INDEX_STATUS] {
        STATUS_SUCCESS => {}
        STATUS_BUSY => return Err(AppError::DeviceBusy),
        STATUS_NOT_SUPPORTED => return Err(AppError::UnsupportedDevice),
        STATUS_FAILURE | STATUS_TIMEOUT => return Err(AppError::UnexpectedResponse),
        _ => return Err(AppError::UnexpectedResponse),
    }

    if report[INDEX_COMMAND_CLASS] != expected_command_class {
        return Err(AppError::UnexpectedResponse);
    }

    if report[INDEX_COMMAND_ID] != expected_command_id {
        return Err(AppError::UnexpectedResponse);
    }

    Ok(())
}

pub fn validate_power_response(report: &[u8], expected_command_id: u8) -> Result<(), AppError> {
    validate_response(report, CMD_CLASS_POWER, expected_command_id)?;
    Ok(())
}

pub fn parse_battery_raw(report: &[u8]) -> Result<u8, AppError> {
    validate_power_response(report, CMD_GET_BATTERY)?;
    Ok(report[INDEX_POWER_VALUE])
}

pub fn parse_charging(report: &[u8]) -> Result<bool, AppError> {
    validate_power_response(report, CMD_GET_CHARGING)?;
    Ok(report[INDEX_POWER_VALUE] != 0)
}
