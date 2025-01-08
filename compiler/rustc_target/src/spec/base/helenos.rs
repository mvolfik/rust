use crate::spec::{
    crt_objects, cvs, Cc, LinkOutputKind, LinkerFlavor, Lld, PanicStrategy, RelroLevel,
    TargetOptions,
};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "helenos".into(),
        // families: cvs!["unix"], -- I'm really not sure about this

        dynamic_linking: true,
        crt_static_default: true,
        crt_static_allows_dylibs: true,
        position_independent_executables: true,
        static_position_independent_executables: true,
        linker: Some("/home/volfmatej/.local/share/HelenOS/cross/bin/amd64-helenos-gcc".into()),

        has_rpath: true,
        relro_level: RelroLevel::Full,
        panic_strategy: PanicStrategy::Abort,

        pre_link_objects: crt_objects::new(&[
            (LinkOutputKind::DynamicNoPicExe, &["libstartfiles.a"]),
            (LinkOutputKind::DynamicPicExe, &["libstartfiles.a"]),
            (LinkOutputKind::StaticNoPicExe, &["libstartfiles.a"]),
            (LinkOutputKind::StaticPicExe, &["libstartfiles.a"]),
        ]),
        pre_link_args: TargetOptions::link_args(
            LinkerFlavor::Gnu(Cc::No, Lld::No),
            &["--hash-style=sysv", "-m64", "-nostartfiles"],
        ),
        ..Default::default()
    }
}
