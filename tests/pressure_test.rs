use memguard::pressure::{classify, parse_pressure, PressureLevel, PressureSnapshot};

#[test]
fn test_parse_pressure() {
    let sample = "some avg10=25.00 avg60=10.00 total=123456\nfull avg10=5.00 avg60=2.00 total=654321\n";
    let snap = parse_pressure(sample).unwrap();
    assert_eq!(snap.some_avg10, 25.0);
    assert_eq!(snap.some_avg60, 10.0);
    assert_eq!(snap.full_avg10, 5.0);
    assert_eq!(snap.full_avg60, 2.0);
}

#[test]
fn test_classify() {
    let snap = PressureSnapshot {
        some_avg10: 10.0,
        ..Default::default()
    };
    assert_eq!(
        classify(&snap, 30.0, 70.0, 50.0),
        PressureLevel::Normal
    );

    let snap = PressureSnapshot {
        some_avg10: 40.0,
        ..Default::default()
    };
    assert_eq!(
        classify(&snap, 30.0, 70.0, 50.0),
        PressureLevel::Warning
    );

    let snap = PressureSnapshot {
        some_avg10: 80.0,
        ..Default::default()
    };
    assert_eq!(
        classify(&snap, 30.0, 70.0, 50.0),
        PressureLevel::Critical
    );

    let snap = PressureSnapshot {
        some_avg10: 0.0,
        full_avg10: 60.0,
        ..Default::default()
    };
    assert_eq!(
        classify(&snap, 30.0, 70.0, 50.0),
        PressureLevel::Critical
    );
}
