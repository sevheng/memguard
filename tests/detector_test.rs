use memguard::desktop::DesktopEnvironment;

#[test]
fn test_desktop_environment_enum_variants() {
    assert_eq!(format!("{:?}", DesktopEnvironment::Gnome), "Gnome");
    assert_eq!(format!("{:?}", DesktopEnvironment::Kde), "Kde");
    assert_eq!(format!("{:?}", DesktopEnvironment::Unknown), "Unknown");
}
