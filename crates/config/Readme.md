### Crate containing `Config` data structure

Rust doesn't allow code to be exported statically and dynamically from the same crate.

Since both the shim and the main DLL might need to get information from the config structure, put it in a third crate that both can statically link against.
