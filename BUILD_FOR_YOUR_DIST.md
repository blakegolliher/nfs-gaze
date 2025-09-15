# Building nfs-gaze for Your Distribution

## System Requirements

### Kernel Compatibility

**Minimum Required Kernel**: Linux 2.6.17 or later

The `/proc/self/mountstats` file was introduced in Linux kernel 2.6.17 (June 2006). This feature is essential for nfs-gaze to function properly.

### Supported Distributions

All major Linux distributions released after 2007 are supported:

| Distribution | Minimum Version | Kernel Version | Support Status |
|-------------|-----------------|----------------|----------------|
| RHEL/CentOS | 5.x | 2.6.18 | ✅ Supported |
| RHEL/Rocky/Alma | 6.x+ | 2.6.32+ | ✅ Supported |
| Debian | 4.0 (Etch) | 2.6.18 | ✅ Supported |
| Ubuntu | 6.10+ | 2.6.17+ | ✅ Supported |
| SLES | 10 SP1+ | 2.6.16.46+ | ✅ Supported |
| openSUSE | 10.2+ | 2.6.18+ | ✅ Supported |

### Build Requirements

- Go 1.21 or later (for building from source)
- GNU Make (optional, for automation)
- rpmbuild (for RPM packages)
- dpkg-deb, debhelper (for DEB packages)
- fakeroot (for DEB packages)

## Building from Source

### Quick Build

```bash
# Clone the repository
git clone https://github.com/blakegolliher/nfs-gaze.git
cd nfs-gaze

# Build the binary
go build -o nfs-gaze

# Install system-wide (optional)
sudo install -m 755 nfs-gaze /usr/local/bin/
```

### Optimized Build

```bash
# Build with optimizations and strip debug symbols
go build -ldflags="-s -w" -o nfs-gaze

# Build for specific architecture
GOOS=linux GOARCH=amd64 go build -o nfs-gaze-amd64
GOOS=linux GOARCH=arm64 go build -o nfs-gaze-arm64
```

## Building RPM Package (Red Hat/Fedora/CentOS/RHEL)

### 1. Prepare the Build Environment

```bash
# Install required packages
sudo yum install -y rpm-build golang make

# Create RPM build tree
mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
```

### 2. Create the SPEC File

Create `~/rpmbuild/SPECS/nfs-gaze.spec`:

```spec
Name:           nfs-gaze
Version:        1.0.0
Release:        1%{?dist}
Summary:        Real-time NFS performance monitoring tool

License:        MIT
URL:            https://github.com/blakegolliher/nfs-gaze
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  golang >= 1.21
BuildRequires:  make
Requires:       kernel >= 2.6.17

%description
nfs-gaze is a real-time NFS I/O performance monitoring tool that provides
detailed statistics about NFS operations with per-operation latency tracking.
It reads from /proc/self/mountstats to display IOPS, bandwidth, latency,
and other metrics for comprehensive NFS performance analysis.

%prep
%setup -q

%build
go build -ldflags="-s -w" -o %{name}

%install
rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT%{_bindir}
mkdir -p $RPM_BUILD_ROOT%{_mandir}/man1
install -m 755 %{name} $RPM_BUILD_ROOT%{_bindir}/%{name}
# If you have a man page:
# install -m 644 %{name}.1 $RPM_BUILD_ROOT%{_mandir}/man1/

%clean
rm -rf $RPM_BUILD_ROOT

%files
%defattr(-,root,root,-)
%doc README.md TESTING.md BUILD_FOR_YOUR_DIST.md
%license LICENSE
%{_bindir}/%{name}
# %{_mandir}/man1/%{name}.1*

%changelog
* Thu Jan 15 2025 Your Name <your.email@example.com> - 1.0.0-1
- Initial RPM release
- Real-time NFS performance monitoring
- Support for multiple output formats
```

### 3. Create Source Tarball

```bash
# From your project directory
VERSION=1.0.0
cd ..
tar czf ~/rpmbuild/SOURCES/nfs-gaze-${VERSION}.tar.gz \
    --transform "s/^nfs-gaze/nfs-gaze-${VERSION}/" \
    nfs-gaze/
```

### 4. Build the RPM

```bash
# Build the RPM
cd ~/rpmbuild
rpmbuild -ba SPECS/nfs-gaze.spec

# The built RPM will be in:
# ~/rpmbuild/RPMS/x86_64/nfs-gaze-1.0.0-1.el8.x86_64.rpm
```

### 5. Install the RPM

```bash
sudo rpm -ivh ~/rpmbuild/RPMS/x86_64/nfs-gaze-1.0.0-1.*.rpm

# Or using yum/dnf
sudo yum localinstall ~/rpmbuild/RPMS/x86_64/nfs-gaze-1.0.0-1.*.rpm
```

## Building DEB Package (Debian/Ubuntu)

### 1. Prepare the Build Environment

```bash
# Install required packages
sudo apt-get install -y build-essential debhelper golang-go fakeroot dpkg-dev
```

### 2. Create Package Directory Structure

```bash
# Create the package directory
mkdir -p nfs-gaze-1.0.0/debian
cd nfs-gaze-1.0.0

# Copy source files
cp -r /path/to/nfs-gaze/* .
```

### 3. Create Debian Control Files

Create `debian/control`:

```control
Source: nfs-gaze
Section: admin
Priority: optional
Maintainer: Your Name <your.email@example.com>
Build-Depends: debhelper (>= 9), golang-go (>= 1.21)
Standards-Version: 4.5.0
Homepage: https://github.com/blakegolliher/nfs-gaze

Package: nfs-gaze
Architecture: any
Depends: ${shlibs:Depends}, ${misc:Depends}
Description: Real-time NFS performance monitoring tool
 nfs-gaze is a real-time NFS I/O performance monitoring tool that provides
 detailed statistics about NFS operations with per-operation latency tracking.
 It reads from /proc/self/mountstats to display IOPS, bandwidth, latency,
 and other metrics for comprehensive NFS performance analysis.
 .
 Features:
  - Real-time monitoring of NFS mounts
  - Per-operation latency tracking
  - Configurable update intervals
  - Operation filtering
  - Bandwidth and attribute cache statistics
```

Create `debian/changelog`:

```changelog
nfs-gaze (1.0.0-1) stable; urgency=low

  * Initial Debian package release
  * Real-time NFS performance monitoring
  * Support for multiple output formats

 -- Your Name <your.email@example.com>  Thu, 15 Jan 2025 12:00:00 +0000
```

Create `debian/compat`:

```
9
```

Create `debian/rules`:

```makefile
#!/usr/bin/make -f

%:
	dh $@

override_dh_auto_build:
	go build -ldflags="-s -w" -o nfs-gaze

override_dh_auto_install:
	install -D -m 755 nfs-gaze debian/nfs-gaze/usr/bin/nfs-gaze
	install -D -m 644 README.md debian/nfs-gaze/usr/share/doc/nfs-gaze/README.md
	install -D -m 644 TESTING.md debian/nfs-gaze/usr/share/doc/nfs-gaze/TESTING.md
	install -D -m 644 BUILD_FOR_YOUR_DIST.md debian/nfs-gaze/usr/share/doc/nfs-gaze/BUILD_FOR_YOUR_DIST.md

override_dh_auto_clean:
	rm -f nfs-gaze
```

Make the rules file executable:

```bash
chmod +x debian/rules
```

Create `debian/copyright`:

```
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: nfs-gaze
Source: https://github.com/blakegolliher/nfs-gaze

Files: *
Copyright: 2025 Your Name <your.email@example.com>
License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a
 copy of this software and associated documentation files (the "Software"),
 to deal in the Software without restriction, including without limitation
 the rights to use, copy, modify, merge, publish, distribute, sublicense,
 and/or sell copies of the Software, and to permit persons to whom the
 Software is furnished to do so, subject to the following conditions:
 .
 The above copyright notice and this permission notice shall be included
 in all copies or substantial portions of the Software.
 .
 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
 OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
 THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 DEALINGS IN THE SOFTWARE.
```

### 4. Build the DEB Package

```bash
# Build the package
dpkg-buildpackage -us -uc -b

# Or use debuild
debuild -us -uc -b

# The built package will be in the parent directory:
# ../nfs-gaze_1.0.0-1_amd64.deb
```

### 5. Install the DEB Package

```bash
# Install using dpkg
sudo dpkg -i ../nfs-gaze_1.0.0-1_amd64.deb

# Or using apt
sudo apt install ../nfs-gaze_1.0.0-1_amd64.deb

# Fix any dependency issues
sudo apt-get install -f
```

## Creating a Makefile for Automation

Create a `Makefile` in your project root:

```makefile
NAME := nfs-gaze
VERSION := 1.0.0
GOFLAGS := -ldflags="-s -w"

.PHONY: all build clean test rpm deb install uninstall

all: build

build:
	go build $(GOFLAGS) -o $(NAME)

test:
	go test -v -cover ./...

clean:
	rm -f $(NAME)
	rm -rf dist/
	rm -f *.rpm *.deb
	rm -f coverage.out coverage.html

install: build
	install -D -m 755 $(NAME) $(DESTDIR)/usr/bin/$(NAME)

uninstall:
	rm -f $(DESTDIR)/usr/bin/$(NAME)

# Build distribution packages
dist: dist-rpm dist-deb

dist-rpm:
	mkdir -p dist/rpm
	# Add RPM building commands here
	@echo "RPM package built in dist/rpm/"

dist-deb:
	mkdir -p dist/deb
	# Add DEB building commands here
	@echo "DEB package built in dist/deb/"

# Development helpers
dev-deps:
	go mod download
	go install golang.org/x/tools/cmd/goimports@latest
	go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

fmt:
	go fmt ./...
	goimports -w .

lint:
	golangci-lint run

coverage:
	go test -coverprofile=coverage.out ./...
	go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report generated in coverage.html"
```

## Distribution-Specific Notes

### Red Hat-based Systems (RHEL, CentOS, Fedora, Rocky, Alma)

1. **SELinux Considerations**: The binary should work with default SELinux policies since it only reads from `/proc/self/mountstats`
2. **Systemd Integration**: Consider adding a systemd service file for continuous monitoring
3. **Repository Hosting**: Consider setting up a YUM/DNF repository for easier distribution

### Debian-based Systems (Debian, Ubuntu, Mint)

1. **AppArmor**: No special AppArmor profile needed for basic operation
2. **PPA Repository**: Consider creating a PPA for Ubuntu users
3. **Snap Package**: For Ubuntu, consider creating a snap package for easier distribution

### Container/Kubernetes Deployment

If you need to run nfs-gaze in containers:

```dockerfile
# Dockerfile
FROM golang:1.21-alpine AS builder
WORKDIR /app
COPY . .
RUN go build -ldflags="-s -w" -o nfs-gaze

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/nfs-gaze /usr/local/bin/
ENTRYPOINT ["nfs-gaze"]
```

**Note**: The container must have access to the host's `/proc/self/mountstats`:

```bash
docker run -v /proc:/host/proc:ro nfs-gaze -f /host/proc/self/mountstats
```

## Verification

After installation, verify the package:

```bash
# Check installation
which nfs-gaze
nfs-gaze --help

# Test with actual NFS mounts
sudo nfs-gaze

# For RPM-based systems
rpm -qi nfs-gaze

# For DEB-based systems
dpkg -l | grep nfs-gaze
```

## Troubleshooting

### Common Build Issues

1. **Go version too old**: Ensure Go 1.21+ is installed
   ```bash
   go version
   ```

2. **Missing mountstats file**: Verify kernel support
   ```bash
   ls -la /proc/self/mountstats
   ```

3. **No NFS mounts**: Ensure NFS mounts exist
   ```bash
   mount -t nfs,nfs4
   ```

### Package Installation Issues

1. **RPM dependency issues**:
   ```bash
   sudo yum install -y kernel-headers
   ```

2. **DEB dependency issues**:
   ```bash
   sudo apt-get update
   sudo apt-get install -f
   ```

## Support Matrix

| Feature | Kernel 2.6.17-2.6.31 | Kernel 3.x | Kernel 4.x | Kernel 5.x+ |
|---------|---------------------|------------|------------|-------------|
| Basic mountstats | ✅ | ✅ | ✅ | ✅ |
| NFS v3 stats | ✅ | ✅ | ✅ | ✅ |
| NFS v4 stats | ✅ | ✅ | ✅ | ✅ |
| RDMA transport | ❌ | ❌ | ✅ (4.2+) | ✅ |
| Error counters | ❌ | ✅ | ✅ | ✅ |

## Contributing

When creating packages for new distributions:

1. Test on the target distribution
2. Verify kernel compatibility
3. Document any distribution-specific requirements
4. Submit packaging files to the project repository
5. Consider automated builds using CI/CD

## License

nfs-gaze is distributed under the MIT License. See LICENSE file for details.
