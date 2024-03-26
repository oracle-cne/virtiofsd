{{{$version := printf "%s.%s.%s" .major .minor .patch }}}

%global _name virtiofsd 

Name:	      %{_name}
Version:      {{{$version}}} 
Release:      1%{?dist}
Vendor:	      Oracle America
Summary:      A virtio-fs vhost-user device daemon written in Rust.
Url:          https://gitlab.com/virtio-fs/virtiofsd
# Upstream license specification: Apache-2.0 AND BSD-3-Clause
License:      Apache-2.0 AND BSD-3-Clause

BuildRequires: libselinux-devel
BuildRequires: libseccomp-devel

BuildRequires: rust-toolset

Obsoletes: qemu-virtiofsd

%description
This package provides virtiofsd daemon. This program is a vhost-user backend
that implements the virtio-fs device that is used for sharing a host directory
tree with a guest.

%prep
%setup -q -n %{name}-%{version}

%cargo_prep -V 1

%build
%cargo_build

%install
mkdir -p %{buildroot}%{_libexecdir}
install -D -p -m 0755 target/release/virtiofsd %{buildroot}%{_libexecdir}/virtiofsd
install -D -p -m 0644 50-qemu-virtiofsd.json %{buildroot}%{_datadir}/qemu/vhost-user/50-qemu-virtiofsd.json

%files
%license LICENSE-APACHE LICENSE-BSD-3-Clause THIRD_PARTY_LICENSES.txt olm/SECURITY.md
%doc README.md
%{_libexecdir}/virtiofsd
%{_datadir}/qemu/vhost-user/50-qemu-virtiofsd.json

%changelog
* {{{.changelog_timestamp}}} - {{{$version}}}-1
- Added Oracle-specific build files.
