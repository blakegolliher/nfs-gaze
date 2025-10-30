%global debug_package %{nil}

Name:           nfs-gaze
Version:        0.1.0
Release:        1.%{build_number}%{?dist}
Summary:        Real-time NFS performance monitoring tool

License:        MIT OR Apache-2.0
URL:            https://github.com/blakegolliher/nfs-gaze
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
Requires:       kernel >= 2.6.17

%description
nfs-gaze is a real-time NFS I/O performance monitoring tool that provides
detailed statistics about NFS operations with per-operation latency tracking.
Built with Rust for memory safety and performance. It reads from
/proc/self/mountstats to display IOPS, bandwidth, latency, and other metrics
for comprehensive NFS performance analysis.

%prep
%setup -q

%build
# Build optimized release binary
cargo build --release

%install
rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT%{_bindir}
install -m 755 target/release/%{name} $RPM_BUILD_ROOT%{_bindir}/%{name}

%clean
rm -rf $RPM_BUILD_ROOT

%files
%defattr(-,root,root,-)
%doc README.md BUILD_FOR_YOUR_DIST.md
%license LICENSE
%{_bindir}/%{name}

%changelog
* Wed Oct 29 2025 Blake Golliher Sr <blakegolliher@gmail.com> - 0.1.0-1.%{build_number}
- Initial RPM release
- Real-time NFS performance monitoring
- Built with Rust for memory safety and performance
- Support for multiple output formats
- Per-operation latency tracking
- Automated RPM builds with unique build numbers
