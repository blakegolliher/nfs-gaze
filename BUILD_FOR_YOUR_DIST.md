# Building nfs-gaze for Your Distribution

## System Requirements

### Kernel Compatibility

**Minimum Required Kernel**: Linux 2.6.17 or later

The `/proc/self/mountstats` file was introduced in Linux kernel 2.6.17 (June 2006). This feature is essential for nfs-gaze to function properly.

### Supported Distributions

All major Linux distributions released after 2007 are supported:

| Distribution | Minimum Version | Kernel Version | Support Status |
|-------------|-----------------|----------------|----------------|
| RHEL/CentOS | 5.x | 2.6.18 | Supported |
| RHEL/Rocky/Alma | 6.x+ | 2.6.32+ | Supported |
| Debian | 4.0 (Etch) | 2.6.18 | Supported |
| Ubuntu | 6.10+ | 2.6.17+ | Supported |
| SLES | 10 SP1+ | 2.6.16.46+ | Supported |
| openSUSE | 10.2+ | 2.6.18+ | Supported |

## Building from Source

### Prerequisites

**Build Tools**:
- Rust 1.70 or later
- Cargo (included with Rust)
- Git

**For RPM Building** (RHEL/CentOS/Rocky/Alma):
- rpmbuild
- rust
- cargo

### Quick Build

```bash
# Clone the repository
git clone https://github.com/blakegolliher/nfs-gaze.git
cd nfs-gaze

# Build the binary (debug mode)
cargo build

# Build optimized release binary
cargo build --release

# The binary will be at target/release/nfs-gaze
./target/release/nfs-gaze --help

# Install system-wide (optional)
sudo install -m 755 target/release/nfs-gaze /usr/local/bin/
```

### Cross-Compilation

```bash
# Install additional targets
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-gnu

# Build static binary (recommended for distribution)
cargo build --release --target x86_64-unknown-linux-musl

# Build for ARM64
cargo build --release --target aarch64-unknown-linux-gnu
```

## Building RPM Package (RHEL/CentOS/Rocky/Alma Linux)

### Prerequisites

```bash
# Install required packages
sudo yum install -y rpm-build rust cargo git
```

### Automated Build

The project includes automated RPM building via the Makefile:

```bash
# Build the RPM package
make rpm
```

This will:
1. Set up the RPM build environment (`~/rpmbuild/`)
2. Create a source tarball
3. Build the RPM with a unique timestamp-based build number
4. Output the RPM locations

**Output**:
- Binary RPM: `~/rpmbuild/RPMS/x86_64/nfs-gaze-0.1.0-1.YYYYMMDDHHMMSS.el9.x86_64.rpm`
- Source RPM: `~/rpmbuild/SRPMS/nfs-gaze-0.1.0-1.YYYYMMDDHHMMSS.el9.src.rpm`

### Custom Build Number

You can override the automatic timestamp with a custom build number:

```bash
make rpm BUILD_NUMBER=mybuild001
```

### Installing the RPM

```bash
# Install using rpm
sudo rpm -ivh ~/rpmbuild/RPMS/x86_64/nfs-gaze-0.1.0-*.rpm

# Or using yum/dnf
sudo yum localinstall ~/rpmbuild/RPMS/x86_64/nfs-gaze-0.1.0-*.rpm
```

### Cleaning Build Artifacts

```bash
# Clean all RPM build artifacts
make rpm-clean
```

## Building DEB Package (Debian/Ubuntu)

### Prerequisites

```bash
# Install required packages
sudo apt-get update
sudo apt-get install -y build-essential debhelper fakeroot dpkg-dev git curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Manual DEB Build

#### 1. Create Package Directory Structure

```bash
# Create the package directory
mkdir -p nfs-gaze-0.1.0/debian
cd nfs-gaze-0.1.0

# Copy source files
cp -r /path/to/nfs-gaze/* .
```

#### 2. Create Debian Control Files

Create `debian/control`:

```control
Source: nfs-gaze
Section: admin
Priority: optional
Maintainer: Blake Golliher Sr <blakegolliher@gmail.com>
Build-Depends: debhelper (>= 9), cargo (>= 1.70), rustc (>= 1.70), git
Standards-Version: 4.5.0
Homepage: https://github.com/blakegolliher/nfs-gaze

Package: nfs-gaze
Architecture: any
Depends: ${shlibs:Depends}, ${misc:Depends}
Description: Real-time NFS performance monitoring tool
 nfs-gaze is a real-time NFS I/O performance monitoring tool that provides
 detailed statistics about NFS operations with per-operation latency tracking.
 Built with Rust for memory safety and high performance. It reads from
 /proc/self/mountstats to display IOPS, bandwidth, latency, and other metrics
 for comprehensive NFS performance analysis.
 .
 Features:
  - Memory-safe Rust implementation
  - Real-time monitoring of NFS mounts
  - Per-operation latency tracking
  - Configurable update intervals
  - Operation filtering
  - Bandwidth and attribute cache statistics
  - Zero memory leaks and thread safety
```

Create `debian/changelog`:

```changelog
nfs-gaze (0.1.0-1) stable; urgency=medium

  * Initial Debian package release
  * Real-time NFS performance monitoring
  * Built with Rust for memory safety and performance
  * Per-operation latency tracking
  * Support for multiple output formats

 -- Blake Golliher Sr <blakegolliher@gmail.com>  Wed, 29 Oct 2025 12:00:00 +0000
```

Create `debian/compat`:

```
10
```

Create `debian/rules`:

```makefile
#!/usr/bin/make -f

%:
	dh $@

override_dh_auto_build:
	cargo build --release

override_dh_auto_install:
	install -D -m 755 target/release/nfs-gaze debian/nfs-gaze/usr/bin/nfs-gaze
	install -D -m 644 README.md debian/nfs-gaze/usr/share/doc/nfs-gaze/README.md
	install -D -m 644 BUILD_FOR_YOUR_DIST.md debian/nfs-gaze/usr/share/doc/nfs-gaze/BUILD_FOR_YOUR_DIST.md

override_dh_auto_clean:
	cargo clean || true

override_dh_auto_test:
	cargo test || true
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
Copyright: 2025 Blake Golliher Sr <blakegolliher@gmail.com>
License: MIT OR Apache-2.0
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

#### 3. Build the DEB Package

```bash
# Build the package
dpkg-buildpackage -us -uc -b

# Or use debuild
debuild -us -uc -b

# The built package will be in the parent directory:
# ../nfs-gaze_0.1.0-1_amd64.deb
```

#### 4. Install the DEB Package

```bash
# Install using dpkg
sudo dpkg -i ../nfs-gaze_0.1.0-1_amd64.deb

# Or using apt
sudo apt install ../nfs-gaze_0.1.0-1_amd64.deb

# Fix any dependency issues
sudo apt-get install -f
```

## Makefile Targets

The project includes several useful make targets:

```bash
make build        # Build the release binary
make test         # Run tests
make coverage     # Generate test coverage report
make clean        # Remove built files and RPM artifacts
make install      # Install binary to system
make uninstall    # Remove installed binary
make rpm          # Build RPM package (CentOS/RHEL/Rocky)
make rpm-clean    # Clean RPM build artifacts
make fmt          # Format code
make lint         # Run clippy linter
make dev          # Format, test, and generate coverage
make watch        # Watch for changes and run tests
make help         # Show help message
```

## Verification

After installation, verify the package:

```bash
# Check installation
which nfs-gaze
nfs-gaze --help
nfs-gaze --version

# Test with actual NFS mounts (requires Linux)
nfs-gaze

# Package verification
# For RPM-based systems
rpm -qi nfs-gaze
rpm -ql nfs-gaze

# For DEB-based systems
dpkg -l | grep nfs-gaze
dpkg -L nfs-gaze
```

## Distribution-Specific Notes

### Red Hat-based Systems (RHEL, CentOS, Fedora, Rocky, Alma)

1. **SELinux**: No special policies needed - reads only from `/proc/self/mountstats`
2. **Rust Installation**: Use `dnf install rust cargo` on newer systems, or `yum install rust cargo` on older versions
3. **Static Linking**: Recommended for better compatibility across versions

### Debian-based Systems (Debian, Ubuntu, Mint)

1. **Rust Installation**: Use official rustup installer: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Build Dependencies**: Ensure `build-essential` is installed
3. **Package Signing**: Consider setting up package signing for security

## Troubleshooting

### Build Issues

1. **Rust version too old**:
   ```bash
   rustup update
   cargo --version
   ```

2. **Missing linker**:
   ```bash
   # Ubuntu/Debian
   sudo apt install build-essential

   # RHEL/CentOS
   sudo yum groupinstall "Development Tools"
   ```

3. **Cross-compilation issues**:
   ```bash
   rustup target add x86_64-unknown-linux-musl
   cargo build --release --target x86_64-unknown-linux-musl
   ```

### Runtime Issues

1. **Missing mountstats**: Verify kernel support
   ```bash
   ls -la /proc/self/mountstats
   ```

2. **No NFS mounts**: Ensure NFS mounts exist
   ```bash
   mount -t nfs,nfs4
   ```

3. **Permission denied**: Check file permissions
   ```bash
   ls -la /proc/self/mountstats
   cat /proc/self/mountstats | head
   ```

## Contributing

When creating packages for new distributions:

1. Test on the target distribution with multiple kernel versions
2. Verify Rust toolchain availability
3. Document any distribution-specific requirements
4. Test both static and dynamic linking
5. Consider automated builds using GitHub Actions
6. Submit packaging files to the project repository

## License

nfs-gaze is distributed under the MIT OR Apache-2.0 License. See LICENSE file for details.
