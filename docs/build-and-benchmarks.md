# Builds, packages, and benchmarks

CapturePort targets Linux. Builds require Rust 1.88 or newer, Cargo, a C
compiler, GPUI's Linux graphics development libraries, and system libgphoto2
and libgphoto2_port development files.

## Packages

Run `cd packaging && makepkg` from a repository checkout to build the Arch
package. The PKGBUILD disables makepkg's C LTO option so bundled SQLite links
correctly. `scripts/build-deb.sh` builds a Debian package on a Debian host with
`dpkg-deb` and `dpkg-shlibdeps` installed. `scripts/build-rpm.sh` builds an RPM
on a Fedora host with `rpmbuild` (`rpm-build`) installed, retaining RPM's
automatic shared-library dependency detection. `scripts/build-portable.sh` creates
a relocatable tarball with the binary, launcher, license, and README. The
portable archive still needs the host graphics and libgphoto2 libraries.

Package scripts print their artifact paths. Set `CAPTUREPORT_VERSION` and
`CAPTUREPORT_OUTPUT_DIR` to change the version and output directory. Packages
must be built on the target architecture.

### GitHub Actions builds

The **Linux packages** workflow runs on every branch push and every tag push,
and can also be started manually. Its three x86_64 jobs build on Ubuntu 24.04,
Arch Linux, and Fedora respectively. Download the artifacts from the workflow
run's **Artifacts** section:

- `captureport-debian-x86_64-<version>`: Linux executable,
  portable `.tar.gz`, and `.deb`;
- `captureport-arch-x86_64-<version>`: pacman `.pkg.tar.zst`;
- `captureport-fedora-x86_64-<version>`: `.rpm`.

Each artifact includes `SHA256SUMS` and is retained for 30 days. Numeric release
tags such as `v0.2.0` use the tag version; prerelease/build separators are
normalized to dots for package manager compatibility. Branch builds and other
tag names use `<crate-version>.dev.<run-number>.<short-commit>`.
Artifacts are uploaded to the workflow run; the workflow does not create a
GitHub Release.

The standalone executable is built on Ubuntu 24.04 and requires compatible
host graphics and libgphoto2 libraries. After extracting the Actions artifact,
run `chmod +x captureport-linux-x86_64` before using that executable. The portable
tarball preserves executable permissions. These builds do not bundle system
libraries or validate a compositor or physical camera.

## Synthetic benchmark

The benchmark harness exercises the public backend APIs against a deterministic
synthetic card containing 10,000 photos and 500 videos. The item sizes
represent a 1 TB card without allocating 1 TB of storage. Thumbnail jobs use
placeholder media, so the run does not require ffmpeg or image fixtures.

Run it from the repository root:

```sh
./scripts/benchmark.sh
```

The output is tab separated with a header row. It includes elapsed seconds,
item counts, bytes actually hashed, and throughput for:

- fake source enumeration;
- catalog upsert and lookup;
- template generation;
- deterministic import planning;
- bounded thumbnail queue processing;
- quick BLAKE3 sampling and full BLAKE3 hashing.

The binary is a normal release executable. Compare runs on the same host and
build profile; these measurements are not release performance guarantees.
The 1 TB figure is represented metadata. Only the hash probes read payload
bytes, and the reported byte throughput applies only to those probes.
