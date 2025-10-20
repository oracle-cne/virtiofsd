

# Build with debug info rpm
%global with_debug 0

%if 0%{?with_debug}
%global _dwz_low_mem_die_limit 0
%else
%global debug_package   %{nil}
%endif

%global _name virtiofsd 
%global _buildhost build-ol%{?oraclelinux}-%{?_arch}.oracle.com
%global virtiofsd_json 50-virtiofsd.json

Name:	       %{_name}
Version:       1.13.2
Release:       1%{?dist}
Vendor:	       Oracle America
Summary:       A virtio-fs vhost-user device daemon written in Rust.
Url:           https://gitlab.com/virtio-fs/virtiofsd
# Upstream license specification: Apache-2.0 AND BSD-3-Clause
License:       Apache-2.0 AND BSD-3-Clause
Source0:       %{name}-%{version}.tar.bz2

BuildRequires: libcap-ng-devel
BuildRequires: libseccomp-devel

BuildRequires: rust-toolset

Obsoletes:     qemu-virtiofsd

%description
This package provides virtiofsd daemon. This program is a vhost-user backend
that implements the virtio-fs device that is used for sharing a host directory
tree with a guest.

%prep
%setup -q -n %{name}-%{version}

%build
cargo build --release

%install
mkdir -p %{buildroot}%{_libexecdir}
install -D -p -m 0755 target/release/virtiofsd %{buildroot}%{_libexecdir}/virtiofsd
install -D -p -m 0644 %{virtiofsd_json} %{buildroot}%{_datadir}/qemu/vhost-user/%{virtiofsd_json}

%files
%license LICENSE-APACHE LICENSE-BSD-3-Clause THIRD_PARTY_LICENSES.txt olm/SECURITY.md
%doc README.md
%{_libexecdir}/virtiofsd
%{_datadir}/qemu/vhost-user/%{virtiofsd_json}

%changelog
* Mon Oct 20 2025 Oracle Cloud Native Environment Authors <noreply@oracle.com> - 1.13.2-1
- Added Oracle-specific build files.
