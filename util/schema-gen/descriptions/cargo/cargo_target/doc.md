The `doc` field indicates whether or not the target is included in the
documentation generated by [`cargo doc`](https://doc.rust-lang.org/cargo/commands/cargo-doc.html) by default. The default is `true` for
libraries and binaries.

> **Note**: The binary will be skipped if its name is the same as the lib
> target.