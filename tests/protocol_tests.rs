use razer_bat::{
    battery::raw_battery_to_percentage,
    device::{RAZER_VID, RazerHidCandidate, select_supported_device},
    error::AppError,
    protocol::{
        CMD_CLASS_POWER, CMD_GET_BATTERY, INDEX_CHECKSUM, INDEX_COMMAND_CLASS, INDEX_COMMAND_ID,
        INDEX_DATA_SIZE, INDEX_POWER_VALUE, INDEX_PROTOCOL_TYPE, INDEX_REMAINING_PACKETS_LSB,
        INDEX_REMAINING_PACKETS_MSB, INDEX_STATUS, INDEX_TRANSACTION_ID, POWER_REPORT_DATA_SIZE,
        PROTOCOL_TYPE, RAZER_REPORT_LEN, STATUS_BUSY, STATUS_NEW_COMMAND, STATUS_SUCCESS,
        WINDOWS_FEATURE_REPORT_LEN, build_get_battery_report, build_get_charging_report,
        calculate_checksum, from_windows_feature_report, parse_battery_raw, parse_charging,
        set_checksum, to_windows_feature_report, validate_response,
    },
};

#[test]
fn report_lengths_are_correct() {
    assert_eq!(RAZER_REPORT_LEN, 90);
    assert_eq!(WINDOWS_FEATURE_REPORT_LEN, 91);

    let report = build_get_battery_report(0x1F);
    let feature_report = to_windows_feature_report(&report);

    assert_eq!(report.len(), RAZER_REPORT_LEN);
    assert_eq!(feature_report.len(), WINDOWS_FEATURE_REPORT_LEN);
}

#[test]
fn get_battery_report_places_power_command() {
    let report = build_get_battery_report(0x1F);

    assert_eq!(report[INDEX_COMMAND_CLASS], CMD_CLASS_POWER);
    assert_eq!(report[INDEX_COMMAND_ID], CMD_GET_BATTERY);
}

#[test]
fn get_charging_report_places_power_command() {
    let report = build_get_charging_report(0x1F);

    assert_eq!(report[INDEX_COMMAND_CLASS], CMD_CLASS_POWER);
    assert_eq!(
        report[INDEX_COMMAND_ID],
        razer_bat::protocol::CMD_GET_CHARGING
    );
}

#[test]
fn power_reports_use_openrazer_single_packet_layout() {
    let report = build_get_battery_report(0x1F);

    assert_eq!(report[INDEX_STATUS], STATUS_NEW_COMMAND);
    assert_eq!(report[INDEX_TRANSACTION_ID], 0x1F);
    assert_eq!(report[INDEX_REMAINING_PACKETS_MSB], 0);
    assert_eq!(report[INDEX_REMAINING_PACKETS_LSB], 0);
    assert_eq!(report[INDEX_PROTOCOL_TYPE], PROTOCOL_TYPE);
    assert_eq!(report[INDEX_DATA_SIZE], POWER_REPORT_DATA_SIZE);
}

#[test]
fn checksum_uses_bytes_two_through_eighty_seven() {
    let mut report = [0; RAZER_REPORT_LEN];
    report[2] = 0xAA;
    report[3] = 0x0F;
    report[87] = 0x55;
    report[88] = 0xFF;

    assert_eq!(calculate_checksum(&report).unwrap(), 0xF0);
}

#[test]
fn report_builder_stores_checksum() {
    let report = build_get_battery_report(0x1F);

    assert_eq!(report[INDEX_CHECKSUM], calculate_checksum(&report).unwrap());
}

#[test]
fn parser_rejects_invalid_length() {
    let report = [0; RAZER_REPORT_LEN - 1];

    assert!(matches!(
        calculate_checksum(&report),
        Err(AppError::InvalidReportLength {
            expected: RAZER_REPORT_LEN,
            actual
        }) if actual == RAZER_REPORT_LEN - 1
    ));
}

#[test]
fn parser_rejects_invalid_checksum() {
    let mut report = successful_battery_response(0x80);
    report[INDEX_CHECKSUM] ^= 0x01;

    assert_eq!(
        validate_response(&report, CMD_CLASS_POWER, CMD_GET_BATTERY),
        Err(AppError::InvalidChecksum)
    );
}

#[test]
fn parser_rejects_wrong_command_class() {
    let mut report = successful_battery_response(0x80);
    report[INDEX_COMMAND_CLASS] = 0x99;
    set_checksum(&mut report).unwrap();

    assert_eq!(
        validate_response(&report, CMD_CLASS_POWER, CMD_GET_BATTERY),
        Err(AppError::UnexpectedResponse)
    );
}

#[test]
fn parser_rejects_wrong_command_id() {
    let mut report = successful_battery_response(0x80);
    report[INDEX_COMMAND_ID] = 0x99;
    set_checksum(&mut report).unwrap();

    assert_eq!(
        validate_response(&report, CMD_CLASS_POWER, CMD_GET_BATTERY),
        Err(AppError::UnexpectedResponse)
    );
}

#[test]
fn parser_rejects_busy_status() {
    let mut report = successful_battery_response(0x80);
    report[INDEX_STATUS] = STATUS_BUSY;
    set_checksum(&mut report).unwrap();

    assert_eq!(
        validate_response(&report, CMD_CLASS_POWER, CMD_GET_BATTERY),
        Err(AppError::DeviceBusy)
    );
}

#[test]
fn parser_extracts_battery_raw_from_openrazer_arg_one() {
    let report = successful_battery_response(0x80);

    assert_eq!(parse_battery_raw(&report).unwrap(), 0x80);
}

#[test]
fn parser_extracts_charging_state_from_openrazer_arg_one() {
    let charging = successful_charging_response(0x01);
    let not_charging = successful_charging_response(0x00);

    assert!(parse_charging(&charging).unwrap());
    assert!(!parse_charging(&not_charging).unwrap());
}

#[test]
fn parser_treats_any_nonzero_charging_status_as_charging() {
    let report = successful_charging_response(0x02);

    assert!(parse_charging(&report).unwrap());
}

#[test]
fn windows_feature_report_round_trips_report_id_prefix() {
    let report = successful_battery_response(0x80);
    let feature_report = to_windows_feature_report(&report);

    assert_eq!(
        from_windows_feature_report(&feature_report).unwrap(),
        report
    );
}

#[test]
fn battery_conversion_rounds_and_clamps() {
    assert_eq!(raw_battery_to_percentage(0), 0);
    assert_eq!(raw_battery_to_percentage(128), 50);
    assert_eq!(raw_battery_to_percentage(255), 100);
}

#[test]
fn device_selector_prefers_known_supported_devices() {
    let candidates = vec![
        RazerHidCandidate {
            name: "Unknown Razer HID".to_string(),
            vid: RAZER_VID,
            pid: 0xFFFF,
            usage_page: Some(0x0001),
            usage: Some(0x0002),
            path: None,
        },
        RazerHidCandidate {
            name: "Razer DeathAdder V3 Pro Wireless".to_string(),
            vid: RAZER_VID,
            pid: 0x00B7,
            usage_page: Some(0x0001),
            usage: Some(0x0002),
            path: None,
        },
    ];

    let (_, definition) = select_supported_device(&candidates).unwrap();

    assert_eq!(definition.pid, 0x00B7);
}

fn successful_battery_response(raw_value: u8) -> [u8; RAZER_REPORT_LEN] {
    let mut report = build_get_battery_report(0x1F);
    report[INDEX_STATUS] = STATUS_SUCCESS;
    report[INDEX_POWER_VALUE] = raw_value;
    set_checksum(&mut report).unwrap();
    report
}

fn successful_charging_response(raw_value: u8) -> [u8; RAZER_REPORT_LEN] {
    let mut report = build_get_charging_report(0x1F);
    report[INDEX_STATUS] = STATUS_SUCCESS;
    report[INDEX_POWER_VALUE] = raw_value;
    set_checksum(&mut report).unwrap();
    report
}
