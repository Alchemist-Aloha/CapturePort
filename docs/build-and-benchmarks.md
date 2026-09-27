# Builds, packages, and benchmarks

CapturePort targets Linux. Builds require Rust 1.88 or newer, Cargo, a C
compiler, GPUI's Linux graphics development libraries, and system libgphoto2
and libgphoto2_port development files.

## Packages

Run `cd packaging && makepkg` from a repository checkout to build the Arch
package. The PKGBUILD disables makepkg's C LTO option so bundled SQLite links
correctly. `scripts/build-deb.sh` builds a Debian package on a Debian host with
`dpkg-deb` and `dpkg-shlibdeps` installed. `scripts/build-portable.sh` creates
a relocatable tarball with the binary, launcher, license, and README. The
portable archive still needs the host graphics and libgphoto2 libraries.

Package scripts print their artifact paths. Set `CAPTUREPORT_VERSION` and
`CAPTUREPORT_OUTPUT_DIR` to change the version and output directory. Packages
must be built on the target architecture.

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
