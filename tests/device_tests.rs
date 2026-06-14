use std::collections::BTreeSet;

use razer_bat::device::{
    DEVICES, MOUSE_USAGE, MOUSE_USAGE_PAGE, RAZER_VID, RazerHidCandidate,
    display_name_for_candidate, known_device, summarize_hid_candidates,
};

#[test]
fn openrazer_mouse_catalog_has_no_duplicate_pids() {
    let mut seen = BTreeSet::new();

    for device in DEVICES {
        assert_eq!(device.vid, RAZER_VID);
        assert!(
            seen.insert(device.pid),
            "duplicate device pid 0x{:04X}",
            device.pid
        );
    }
}

#[test]
fn catalog_contains_recent_openrazer_battery_mice() {
    let deathadder = known_device(RAZER_VID, 0x00B7).unwrap();
    assert_eq!(deathadder.name, "Razer DeathAdder V3 Pro (Wireless)");
    assert_eq!(deathadder.transaction_id, 0x1F);
    assert!(deathadder.supports_battery);
    assert!(deathadder.supports_charging);

    let viper = known_device(RAZER_VID, 0x00C1).unwrap();
    assert_eq!(viper.name, "Razer Viper V3 Pro (Wireless)");
    assert_eq!(viper.transaction_id, 0x1F);
    assert!(viper.supports_battery);
    assert!(viper.supports_charging);
}

#[test]
fn catalog_keeps_aa_battery_mice_without_charging_support() {
    let orochi = known_device(RAZER_VID, 0x0094).unwrap();

    assert_eq!(orochi.name, "Razer Orochi V2 (Receiver)");
    assert!(orochi.supports_battery);
    assert!(!orochi.supports_charging);
}

#[test]
fn catalog_keeps_wired_only_mice_unsupported_for_battery() {
    let deathadder = known_device(RAZER_VID, 0x00B2).unwrap();

    assert_eq!(deathadder.name, "Razer DeathAdder V3");
    assert!(!deathadder.supports_battery);
    assert!(!deathadder.supports_charging);
}

#[test]
fn catalog_mouse_entries_use_generic_mouse_usage() {
    for device in DEVICES {
        assert_eq!(device.usage_page, Some(MOUSE_USAGE_PAGE));
        assert_eq!(device.usage, Some(MOUSE_USAGE));
    }
}

#[test]
fn known_device_name_overrides_hid_product_string_for_display() {
    let candidate = RazerHidCandidate {
        name: "USB Receiver".to_string(),
        vid: RAZER_VID,
        pid: 0x00B7,
        usage_page: Some(MOUSE_USAGE_PAGE),
        usage: Some(MOUSE_USAGE),
    };

    assert_eq!(
        display_name_for_candidate(&candidate),
        "Razer DeathAdder V3 Pro (Wireless)"
    );
}

#[test]
fn summaries_group_distinct_hid_interfaces_for_one_device() {
    let candidates = vec![
        RazerHidCandidate {
            name: "USB Receiver".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            usage_page: Some(0x0001),
            usage: Some(0x0002),
        },
        RazerHidCandidate {
            name: "USB Receiver".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            usage_page: Some(0x000C),
            usage: Some(0x0001),
        },
        RazerHidCandidate {
            name: "USB Receiver".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            usage_page: Some(0x0001),
            usage: Some(0x0002),
        },
    ];

    let summaries = summarize_hid_candidates(&candidates);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].name, "Razer Basilisk V3 Pro (Wireless)");
    assert_eq!(summaries[0].pid, 0x00AB);
    assert_eq!(summaries[0].usages.len(), 2);
}
