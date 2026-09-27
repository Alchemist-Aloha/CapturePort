# Local compatibility patch

This directory vendors `proc-macro-error2` 2.0.1 from crates.io (registry
checksum `11ec05c52be0a07b08061f7dd003e7d7092e0472bc731b4af7bb1ef876109802`).
It is used by GPUI 0.2.2 through `stacksafe-macro` 0.1.4. The upstream project
is archived, so this workspace patches the existing dependency in place.

The only upstream source change is in `src/lib.rs`: `extern crate proc_macro;`
became `pub extern crate proc_macro;`. This resolves Rust future-incompatibility
warning E0365 for its public re-export in `__export` without changing its API.
The original MIT and Apache-2.0 license files are included here.
