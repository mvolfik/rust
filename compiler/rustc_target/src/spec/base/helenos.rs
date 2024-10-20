use crate::spec::{cvs, LinkerFlavor, Cc, Lld, RelroLevel, TargetOptions};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "helenos".into(),
        dynamic_linking: true,
        families: cvs!["unix"],
        crt_static_default: true,
        crt_static_allows_dylibs: true,
        has_thread_local: true,
        has_rpath: true,
        relro_level: RelroLevel::Full,
        pre_link_args: TargetOptions::link_args(
            LinkerFlavor::Gnu(Cc::No, Lld::No),
            &[
                "--hash-style=sysv", "-m64"
            ],
        ),
        ..Default::default()
    }
}
