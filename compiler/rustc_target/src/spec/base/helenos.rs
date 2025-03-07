use crate::spec::{PanicStrategy, RelroLevel, TargetOptions};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "helenos".into(),

        dynamic_linking: true,
        crt_static_default: true,
        crt_static_allows_dylibs: true,
        position_independent_executables: true,
        static_position_independent_executables: true,

        has_rpath: true,
        relro_level: RelroLevel::Full,
        panic_strategy: PanicStrategy::Abort,

        ..Default::default()
    }
}
