fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
        && std::env::var("PROFILE").as_deref() == Ok("debug")
    {
        // Clap's generated debug command builder exceeds the default 1 MiB stack:
        // https://github.com/clap-rs/clap/issues/5134
        println!("cargo:rustc-link-arg-bin=jirac=/STACK:8388608");
    }
}
