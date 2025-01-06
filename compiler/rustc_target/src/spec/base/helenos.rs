use crate::spec::{
    crt_objects, cvs, Cc, LinkOutputKind, LinkerFlavor, Lld, PanicStrategy, RelroLevel,
    TargetOptions,
};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "helenos".into(),

        dynamic_linking: true,
        crt_static_default: true,
        crt_static_allows_dylibs: true,
        position_independent_executables: true,

        has_thread_local: false,
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
