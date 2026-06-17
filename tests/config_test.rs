use memguard::config::Config;
use std::io::Write;

#[test]
fn test_default_config_values() {
    let cfg = Config::default();
    assert_eq!(cfg.pressure.poll_ms, 500);
    assert_eq!(cfg.policy.kill_delay_seconds, 5);
    assert!(cfg.desktop.supported.contains(&"gnome".to_string()));
}

#[test]
fn test_load_config_from_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(
        tmp,
        r#"
[pressure]
poll_ms = 1000

[policy]
kill_delay_seconds = 3

[desktop]
supported = ["gnome"]
"#
    )
    .unwrap();

    let cfg = Config::load(tmp.path()).unwrap();
    assert_eq!(cfg.pressure.poll_ms, 1000);
    assert_eq!(cfg.policy.kill_delay_seconds, 3);
    assert_eq!(cfg.desktop.supported, vec!["gnome".to_string()]);
}

#[test]
fn test_load_or_default_missing_uses_default() {
    let path = std::path::Path::new("/nonexistent/memguard/config.toml");
    let cfg = Config::load_or_default(path);
    assert_eq!(cfg, Config::default());
}

#[test]
fn test_load_or_default_valid_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(
        tmp,
        r#"
[pressure]
poll_ms = 1234

[policy]
kill_delay_seconds = 7
"#
    )
    .unwrap();

    let cfg = Config::load_or_default(tmp.path());
    assert_eq!(cfg.pressure.poll_ms, 1234);
    assert_eq!(cfg.policy.kill_delay_seconds, 7);
    // Missing fields fall back to defaults.
    assert_eq!(cfg.policy.recovery_seconds, 10);
    assert_eq!(cfg.desktop.session_dir, "/run/systemd/sessions");
}
