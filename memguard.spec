Name:           memguard
Version:        0.1.0
Release:        1%{?dist}
Summary:        Linux desktop memory pressure daemon
License:        MIT
URL:            https://github.com/<user>/memguard
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  systemd-rpm-macros

%description
memguard monitors memory pressure on Linux desktops and applies cgroup-level
mitigations (freeze / kill) to protect the active shell and foreground app.

%prep
%autosetup -n %{name}-%{version}

%build
cd memguard
cargo build --release

%install
install -D -p -m 0755 memguard/target/release/memguard %{buildroot}%{_bindir}/memguard
install -D -p -m 0644 memguard.service %{buildroot}%{_unitdir}/memguard.service
install -D -p -m 0644 dbus/memguard.conf %{buildroot}%{_datadir}/dbus-1/system.d/memguard.conf
install -D -p -m 0644 config.toml %{buildroot}%{_sysconfdir}/memguard/config.toml
install -D -p -m 0644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -D -p -m 0644 LICENSE %{buildroot}%{_docdir}/%{name}/LICENSE
install -d -m 0755 %{buildroot}%{_docdir}/%{name}/docs
cp -r docs/* %{buildroot}%{_docdir}/%{name}/docs/

%post
%systemd_post memguard.service

%preun
%systemd_preun memguard.service

%postun
%systemd_postun memguard.service

%files
%license LICENSE
%doc README.md
%{_bindir}/memguard
%{_unitdir}/memguard.service
%{_datadir}/dbus-1/system.d/memguard.conf
%config(noreplace) %{_sysconfdir}/memguard/config.toml
%{_docdir}/%{name}/docs/
