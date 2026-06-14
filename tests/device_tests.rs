use std::collections::BTreeSet;

use razer_bat::device::{
    DEVICES, MOUSE_USAGE, MOUSE_USAGE_PAGE, OPENRAZER_DEFAULT_INTERFACE, OPENRAZER_INTERFACE_3,
    RAZER_VID, RazerHidCandidate, display_name_for_candidate, known_device,
    ranked_supported_battery_candidates, select_supported_device, summarize_hid_candidates,
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
fn catalog_keeps_openrazer_interface_preferences() {
    let basilisk_v3_pro = known_device(RAZER_VID, 0x00AB).unwrap();
    assert_eq!(
        basilisk_v3_pro.preferred_interface_number,
        Some(OPENRAZER_DEFAULT_INTERFACE)
    );

    let basilisk_v3_35k = known_device(RAZER_VID, 0x00CB).unwrap();
    assert_eq!(
        basilisk_v3_35k.preferred_interface_number,
        Some(OPENRAZER_INTERFACE_3)
    );
}

#[test]
fn known_device_name_overrides_hid_product_string_for_display() {
    let candidate = RazerHidCandidate {
        name: "USB Receiver".to_string(),
        vid: RAZER_VID,
        pid: 0x00B7,
        interface_number: Some(OPENRAZER_DEFAULT_INTERFACE),
        usage_page: Some(MOUSE_USAGE_PAGE),
        usage: Some(MOUSE_USAGE),
        path: None,
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
            interface_number: Some(0),
            usage_page: Some(0x0001),
            usage: Some(0x0002),
            path: None,
        },
        RazerHidCandidate {
            name: "USB Receiver".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            interface_number: Some(1),
            usage_page: Some(0x000C),
            usage: Some(0x0001),
            path: None,
        },
        RazerHidCandidate {
            name: "USB Receiver".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            interface_number: Some(0),
            usage_page: Some(0x0001),
            usage: Some(0x0002),
            path: None,
        },
    ];

    let summaries = summarize_hid_candidates(&candidates);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].name, "Razer Basilisk V3 Pro (Wireless)");
    assert_eq!(summaries[0].pid, 0x00AB);
    assert_eq!(summaries[0].usages.len(), 2);
}

#[test]
fn selector_prefers_openrazer_interface_over_mouse_usage() {
    let candidates = vec![
        RazerHidCandidate {
            name: "Razer Basilisk V3 Pro".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            interface_number: Some(1),
            usage_page: Some(MOUSE_USAGE_PAGE),
            usage: Some(MOUSE_USAGE),
            path: None,
        },
        RazerHidCandidate {
            name: "Razer Basilisk V3 Pro".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            interface_number: Some(OPENRAZER_DEFAULT_INTERFACE),
            usage_page: Some(MOUSE_USAGE_PAGE),
            usage: Some(0x0000),
            path: None,
        },
    ];

    let (candidate, definition) = select_supported_device(&candidates).unwrap();

    assert_eq!(definition.pid, 0x00AB);
    assert_eq!(
        candidate.interface_number,
        Some(OPENRAZER_DEFAULT_INTERFACE)
    );
}

#[test]
fn selector_falls_back_to_mouse_usage_when_preferred_interface_is_absent() {
    let candidates = vec![
        RazerHidCandidate {
            name: "Razer Basilisk V3 Pro".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            interface_number: Some(2),
            usage_page: Some(0x000C),
            usage: Some(0x0001),
            path: None,
        },
        RazerHidCandidate {
            name: "Razer Basilisk V3 Pro".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            interface_number: Some(1),
            usage_page: Some(MOUSE_USAGE_PAGE),
            usage: Some(MOUSE_USAGE),
            path: None,
        },
    ];

    let (candidate, _) = select_supported_device(&candidates).unwrap();

    assert_eq!(candidate.interface_number, Some(1));
    assert_eq!(candidate.usage_page, Some(MOUSE_USAGE_PAGE));
    assert_eq!(candidate.usage, Some(MOUSE_USAGE));
}

#[test]
fn ranked_candidates_keep_all_supported_interfaces_for_probe_fallback() {
    let candidates = vec![
        RazerHidCandidate {
            name: "Razer Basilisk V3 Pro".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            interface_number: Some(1),
            usage_page: Some(MOUSE_USAGE_PAGE),
            usage: Some(MOUSE_USAGE),
            path: None,
        },
        RazerHidCandidate {
            name: "Razer Basilisk V3 Pro".to_string(),
            vid: RAZER_VID,
            pid: 0x00AB,
            interface_number: Some(OPENRAZER_DEFAULT_INTERFACE),
            usage_page: Some(MOUSE_USAGE_PAGE),
            usage: Some(0x0000),
            path: None,
        },
    ];

    let ranked = ranked_supported_battery_candidates(&candidates);

    assert_eq!(ranked.len(), 2);
    assert_eq!(
        ranked[0].0.interface_number,
        Some(OPENRAZER_DEFAULT_INTERFACE)
    );
    assert_eq!(ranked[1].0.interface_number, Some(1));
}
