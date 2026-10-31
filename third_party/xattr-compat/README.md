# GPUI xattr compatibility

GPUI 0.2.2 → gpui_http_client → zed-async-tar 0.5.0-zed requires
`xattr ^0.2`. That old crate references Linux's removed `libc::ENOATTR`
alias and no longer builds with current libc.

This shim satisfies that legacy dependency and re-exports current `xattr`
1.7.0 rather than pinning libc or maintaining a fork of filesystem code.
The tar crate's sole xattr call is `xattr::set`, whose signature is compatible.
The legacy `unsupported` feature is forwarded unchanged.

Remove the patch when GPUI's tar dependency supports current xattr directly.
