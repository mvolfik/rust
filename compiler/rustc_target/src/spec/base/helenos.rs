use crate::spec::{Cc, LinkerFlavor, Lld, PanicStrategy, RelroLevel, TargetOptions};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "helenos".into(),
        // families: cvs!["unix"], // I'm really not sure about this
        dynamic_linking: true,
        crt_static_default: true,
        crt_static_allows_dylibs: true,
        position_independent_executables: true,
        static_position_independent_executables: true,
        linker: Some("amd64-helenos-gcc".into()),

        has_rpath: true,
        relro_level: RelroLevel::Full,
        panic_strategy: PanicStrategy::Abort,

        pre_link_args: TargetOptions::link_args(
            LinkerFlavor::Gnu(Cc::Yes, Lld::No),
            &[
                // "--hash-style=sysv", // this doesn't seem necessary when using the helenos linker, since it's already the default
                "-m64",
            ],
        ),
        ..Default::default()
    }
}
