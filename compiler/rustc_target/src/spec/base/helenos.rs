use crate::spec::{PanicStrategy, RelroLevel, TargetOptions};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "helenos".into(),

        // FIXME: without these two flags, my binaries contained R_386_RELATIVE relocations,
        // which caused the loader to segfault. I should figure out why this is happening,
        // and if the error is in the binary or in the loader. Now, we get R_386_JUMP_SLOT
        // relocations, which work fine.
        crt_static_default: true,
        crt_static_allows_dylibs: true,

        dynamic_linking: true,
        position_independent_executables: true,
        static_position_independent_executables: true,
        no_default_libraries: false,

        has_rpath: true,
        relro_level: RelroLevel::Full,
        panic_strategy: PanicStrategy::Abort,

        ..Default::default()
    }
}
