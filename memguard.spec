Name:           memguard
Version:        0.1.0
Release:        1%{?dist}
Summary:        Linux desktop memory pressure daemon
License:        MIT
URL:            https://github.com/<user>/memguard
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  systemd-rpm-macros

%description
memguard monitors memory pressure on Linux desktops and applies cgroup-level
mitigations (freeze / kill) to protect the active shell and foreground app.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release

%install
install -D -p -m 0755 target/release/memguard %{buildroot}%{_bindir}/memguard
install -D -p -m 0644 memguard.service %{buildroot}%{_unitdir}/memguard.service
install -D -p -m 0644 dbus/memguard.conf %{buildroot}%{_datadir}/dbus-1/system.d/memguard.conf
install -D -p -m 0644 config.toml %{buildroot}%{_sysconfdir}/memguard/config.toml
install -d -m 0755 %{buildroot}%{_docdir}/%{name}/docs
cp -r docs/* %{buildroot}%{_docdir}/%{name}/docs/
install -D -p -m 0755 system-tune/memguard-system-tune %{buildroot}%{_bindir}/memguard-system-tune
install -D -p -m 0644 system-tune/memguard-system-tune.service %{buildroot}%{_unitdir}/memguard-system-tune.service
install -D -p -m 0644 system-tune/README.md %{buildroot}%{_docdir}/%{name}-system-tune/README.md
install -D -p -m 0644 LICENSE %{buildroot}%{_docdir}/%{name}-system-tune/LICENSE
install -d -m 0755 %{buildroot}%{_localstatedir}/lib/%{name}-system-tune

%post
%systemd_post memguard.service

%preun
%systemd_preun memguard.service

%postun
%systemd_postun memguard.service

%package system-tune
Summary:        Static system tuning for memguard desktops
BuildArch:      noarch
Requires:       systemd
Requires:       util-linux
Requires:       bash
Recommends:     ananicy-cpp

%description system-tune
One-time system tuning helper for low-end desktops using memguard. Sets the
I/O scheduler to bfq, enables ananicy-cpp, configures zram, runs fstrim, and
adds noatime to fstab.

%post system-tune
%systemd_post memguard-system-tune.service

%preun system-tune
%systemd_preun memguard-system-tune.service

%postun system-tune
%systemd_postun memguard-system-tune.service

%files
%license LICENSE
%doc README.md
%{_bindir}/memguard
%{_unitdir}/memguard.service
%{_datadir}/dbus-1/system.d/memguard.conf
%config(noreplace) %{_sysconfdir}/memguard/config.toml
%{_docdir}/%{name}/docs/

%files system-tune
%license LICENSE
%{_bindir}/memguard-system-tune
%{_unitdir}/memguard-system-tune.service
%{_docdir}/%{name}-system-tune/README.md
%dir %{_localstatedir}/lib/%{name}-system-tune
