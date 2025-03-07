# `*-unknown-helenos`

**Tier: 3**

Targets for [HelenOS](https://www.helenos.org).
The [Hermit] unikernel target allows compiling your applications into self-contained, specialized unikernel images that can be run in small virtual machines.

[Hermit]: https://github.com/hermit-os

Target triplets available so far:

- `x86_64-unknown-helenos`
- `i686-unknown-helenos`

## Target maintainers

- Matěj Volf ([@mvolfik](https://github.com/mvolfik))

## Requirements

These targets only support cross-compilation. The targets do support std, although support of some platform features (filesystem, networking) may be limited.

When building binaries for this target, the Hermit unikernel is built from scratch.
The application developer themselves specializes the target and sets corresponding expectations.

The Hermit targets follow Linux's `extern "C"` calling convention.

Hermit binaries have the ELF format.

## Building

### HelenOS toolchain setup

For compilation of standard library, you need to build the HelenOS toolchain (because Rust needs to use `*-helenos-gcc` as linker) and shared libraries. See [this HelenOS wiki page](https://www.helenos.org/wiki/UsersGuide/CompilingFromSource#a2.Buildasupportedcross-compiler) for instruction on setting up the build. At the end of step 4 (_Configure and build_), invoke `ninja export-dev` to build the shared libraries.

## Building the target

When you have the HelenOS toolchain set up and installed in your path, you can build the Rust toolchain using the standard procedure. See [rustc dev guide](https://rustc-dev-guide.rust-lang.org/building/how-to-build-and-run.html).

## Building Rust programs

No special setup is needed. Simply use the toolchain you build above and run `cargo build --target <arch>-unknown-helenos`.

## Testing

Running the Rust has not been attempted yet.

## Cross-compilation toolchains and C code

You should be able to cross-compile and link any needed C code using `<arch>-helenos-gcc` that you built above.
