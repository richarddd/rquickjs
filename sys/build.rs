#![allow(clippy::uninlined_format_args)]
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{self},
};

// WASI logic lifted from https://github.com/bytecodealliance/javy/blob/61616e1507d2bf896f46dc8d72687273438b58b2/crates/quickjs-wasm-sys/build.rs#L18

const WASI_SDK_VERSION_MAJOR: usize = 24;
const WASI_SDK_VERSION_MINOR: usize = 0;

fn download_wasi_sdk() -> PathBuf {
    let mut wasi_sdk_dir: PathBuf = env::var("OUT_DIR").unwrap().into();
    wasi_sdk_dir.push("wasi-sdk");

    fs::create_dir_all(&wasi_sdk_dir).unwrap();

    let major_version = WASI_SDK_VERSION_MAJOR;
    let minor_version = WASI_SDK_VERSION_MINOR;

    let mut archive_path = wasi_sdk_dir.clone();
    archive_path.push(format!("wasi-sdk-{major_version}-{minor_version}.tar.gz"));

    println!("SDK tar: {archive_path:?}");

    // Download archive if necessary
    if !archive_path.try_exists().unwrap() {
        let file_suffix = match (env::consts::OS, env::consts::ARCH) {
            ("linux", "x86") | ("linux", "x86_64") => "x86_64-linux",
            ("linux", "aarch64") => "arm64-linux",
            ("macos", "x86") | ("macos", "x86_64") => "x86_64-macos",
            ("macos", "aarch64") => "arm64-macos",
            ("windows", "x86") | ("windows", "x86_64") => "x86_64-windows",
            ("windows", "aarch64") => "arm64-windows",
            other => panic!("Unsupported platform tuple {:?}", other),
        };

        let uri = format!("https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-{major_version}/wasi-sdk-{major_version}.{minor_version}-{file_suffix}.tar.gz");

        println!("Downloading WASI SDK archive from {uri} to {archive_path:?}");

        let output = process::Command::new("curl")
            .args([
                "--location",
                "-o",
                archive_path.to_string_lossy().as_ref(),
                uri.as_ref(),
            ])
            .output()
            .expect("failed to download the WASI SDK with curl");
        println!("curl output: {}", String::from_utf8_lossy(&output.stdout));
        println!("curl err: {}", String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            panic!(
                "curl WASI SDK failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    let mut test_binary = wasi_sdk_dir.clone();
    test_binary.extend(["bin", "wasm-ld"]);
    // Extract archive if necessary
    if !test_binary.try_exists().unwrap() {
        println!("Extracting WASI SDK archive {archive_path:?}");
        let output = process::Command::new("tar")
            .args([
                "-zxf",
                archive_path.to_string_lossy().as_ref(),
                "--strip-components",
                "1",
            ])
            .current_dir(&wasi_sdk_dir)
            .output()
            .unwrap();
        if !output.status.success() {
            panic!(
                "Unpacking WASI SDK failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    wasi_sdk_dir
}

fn get_wasi_sdk_path() -> PathBuf {
    std::env::var_os("WASI_SDK")
        .map(PathBuf::from)
        .unwrap_or_else(download_wasi_sdk)
}

fn main() {
    #[cfg(feature = "logging")]
    pretty_env_logger::init();

    let features = [
        "bindgen",
        "update-bindings",
        "dump-bytecode",
        "dump-gc",
        "dump-gc-free",
        "dump-free",
        "dump-leaks",
        "dump-mem",
        "dump-objects",
        "dump-atoms",
        "dump-shapes",
        "dump-module-resolve",
        "dump-promise",
        "dump-read-object",
        "disable-assertions",
    ];

    for feature in &features {
        println!("cargo:rerun-if-env-changed={}", feature_to_cargo(feature));
    }
    println!("cargo:rerun-if-env-changed=CARGO_CFG_SANITIZE");

    println!("cargo:rerun-if-changed=compat_original.c");
    println!("cargo:rerun-if-changed=compat_original.h");
    println!("cargo:rerun-if-changed=quickjs.bind.h");

    let flavor = Flavor::resolve();
    let src_dir = flavor.src_dir;

    let out_dir = env::var("OUT_DIR").expect("No OUT_DIR env var is set by cargo");
    let out_dir = Path::new(&out_dir);

    let header_files = flavor.headers;
    let source_files = flavor.sources;

    // The original QuickJS expects `CONFIG_VERSION` to be defined by the build
    // system from its `VERSION` file (quickjs-ng defines its version in a
    // header, so this is only needed for the `quickjs-og` flavor).
    let config_version = if flavor.original {
        let version = fs::read_to_string(src_dir.join("VERSION"))
            .expect("Unable to read quickjs-original VERSION file");
        Some(format!("\"{}\"", version.trim()))
    } else {
        None
    };

    let mut defines: Vec<(String, Option<&str>)> = vec![("_GNU_SOURCE".into(), None)];

    #[cfg(feature = "disable-assertions")]
    defines.push(("NDEBUG".into(), None));

    if let Some(config_version) = config_version.as_deref() {
        defines.push(("CONFIG_VERSION".into(), Some(config_version)));
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();

    let mut builder = cc::Build::new();
    builder
        .extra_warnings(false)
        .flag_if_supported("-Wno-implicit-const-int-float-conversion")
        //.flag("-Wno-array-bounds")
        //.flag("-Wno-format-truncation")
        ;

    match env::var("CARGO_CFG_SANITIZE").as_deref() {
        Ok("address") => {
            builder
                .flag("-fsanitize=address")
                .flag("-fno-sanitize-recover=all")
                .flag("-fno-omit-frame-pointer");
        }
        Ok("memory") => {
            builder
                .flag("-fsanitize=memory")
                .flag("-fno-sanitize-recover=all")
                .flag("-fno-omit-frame-pointer");
        }
        Ok("thread") => {
            builder
                .flag("-fsanitize=thread")
                .flag("-fno-sanitize-recover=all")
                .flag("-fno-omit-frame-pointer");
        }
        Ok(x) => println!("cargo:warning=Unsupported sanitize_option: '{x}'"),
        _ => {}
    }

    let mut bindgen_cflags = vec![];

    if target_os == "windows" {
        if target_env == "msvc" {
            env::set_var(
                "CFLAGS",
                "/DWIN32_LEAN_AND_MEAN /std:c11 /experimental:c11atomics",
            );
        } else {
            env::set_var("CFLAGS", "-DWIN32_LEAN_AND_MEAN -std=c11");
        }
    }

    if target_os == "wasi" {
        // pretend we're emscripten - there are already ifdefs that match
        // also, wasi doesn't ahve FE_DOWNWARD or FE_UPWARD
        defines.push(("EMSCRIPTEN".into(), Some("1")));
        defines.push(("FE_DOWNWARD".into(), Some("0")));
        defines.push(("FE_UPWARD".into(), Some("0")));
    }

    for file in source_files.iter().chain(header_files.iter()) {
        fs::copy(src_dir.join(file), out_dir.join(file))
            .expect("Unable to copy source; try 'git submodule update --init'");
    }
    fs::copy("quickjs.bind.h", out_dir.join("quickjs.bind.h")).expect("Unable to copy source");

    // For the original flavor, compile a small compatibility shim that
    // re-exports quickjs-ng-style helpers which the original only provides as
    // `static inline` (and therefore aren't picked up by bindgen). The shim's
    // declarations are appended to the bindgen header so Rust gets bindings.
    if flavor.original {
        fs::copy("compat_original.c", out_dir.join("compat_original.c"))
            .expect("Unable to copy compat_original.c");
        fs::copy("compat_original.h", out_dir.join("compat_original.h"))
            .expect("Unable to copy compat_original.h");

        let bind_header = out_dir.join("quickjs.bind.h");
        let mut contents = fs::read_to_string(&bind_header).unwrap();
        contents.push_str("\n#include \"compat_original.h\"\n");
        fs::write(&bind_header, contents).unwrap();
    }

    if target_os == "wasi" && !matches!(env::var("RQUICKJS_SYS_NO_WASI_SDK").as_deref(), Ok("1")) {
        let wasi_sdk_path = get_wasi_sdk_path();
        if !wasi_sdk_path.try_exists().unwrap() {
            panic!(
                "wasi-sdk not installed in specified path of {}",
                wasi_sdk_path.display()
            );
        }
        env::set_var("CC", wasi_sdk_path.join("bin/clang").to_str().unwrap());
        env::set_var("AR", wasi_sdk_path.join("bin/ar").to_str().unwrap());
        let sysroot = format!(
            "--sysroot={}",
            wasi_sdk_path.join("share/wasi-sysroot").display()
        );
        env::set_var("CFLAGS", &sysroot);
        bindgen_cflags.push(sysroot);
    }

    // generating bindings
    bindgen(
        out_dir,
        out_dir.join("quickjs.bind.h"),
        &defines,
        bindgen_cflags,
        flavor.original,
    );

    for (name, value) in &defines {
        builder.define(name, *value);
    }

    for src in source_files {
        builder.file(out_dir.join(src));
    }

    if flavor.original {
        builder.file(out_dir.join("compat_original.c"));
    }

    builder.compile("libquickjs.a");
}

/// Returns the source directory of the selected QuickJS flavor.
///
/// The `quickjs-ng` and `quickjs-og` features are mutually exclusive;
/// exactly one must be enabled.
/// Everything that differs between the two QuickJS C flavors, resolved once.
struct Flavor {
    /// Whether this is the original (Bellard) flavor. `false` means quickjs-ng.
    original: bool,
    /// Submodule directory holding the C sources.
    src_dir: &'static Path,
    /// Header files to copy into `OUT_DIR`.
    headers: &'static [&'static str],
    /// Source files to compile.
    sources: &'static [&'static str],
}

impl Flavor {
    /// Resolve the selected flavor from cargo features.
    ///
    /// The invalid "both" / "neither" cases are reported to the user via
    /// `compile_error!` in lib.rs (a friendlier diagnostic than a build-script
    /// panic); if both are somehow enabled we default to quickjs-ng so the
    /// build proceeds far enough to surface that error.
    fn resolve() -> Self {
        const NG_HEADERS: &[&str] = &[
            "builtin-array-fromasync.h",
            "builtin-iterator-zip-keyed.h",
            "builtin-iterator-zip.h",
            "cutils.h",
            "dtoa.h",
            "libregexp-opcode.h",
            "libregexp.h",
            "libunicode-table.h",
            "libunicode.h",
            "list.h",
            "quickjs-atom.h",
            "quickjs-opcode.h",
            "quickjs-c-atomics.h",
            "quickjs.h",
        ];
        const NG_SOURCES: &[&str] = &["libregexp.c", "libunicode.c", "quickjs.c", "dtoa.c"];

        // The original lacks the `builtin-*` and `quickjs-c-atomics.h` headers
        // and ships `cutils.c` as a separate translation unit.
        const ORIG_HEADERS: &[&str] = &[
            "cutils.h",
            "dtoa.h",
            "libregexp-opcode.h",
            "libregexp.h",
            "libunicode-table.h",
            "libunicode.h",
            "list.h",
            "quickjs-atom.h",
            "quickjs-opcode.h",
            "quickjs.h",
        ];
        const ORIG_SOURCES: &[&str] = &[
            "cutils.c",
            "libregexp.c",
            "libunicode.c",
            "quickjs.c",
            "dtoa.c",
        ];

        if cfg!(feature = "quickjs-og") && !cfg!(feature = "quickjs-ng") {
            Flavor {
                original: true,
                src_dir: Path::new("quickjs-original"),
                headers: ORIG_HEADERS,
                sources: ORIG_SOURCES,
            }
        } else {
            Flavor {
                original: false,
                src_dir: Path::new("quickjs"),
                headers: NG_HEADERS,
                sources: NG_SOURCES,
            }
        }
    }
}

fn feature_to_cargo(name: impl AsRef<str>) -> String {
    format!("CARGO_FEATURE_{}", feature_to_define(name))
}

fn feature_to_define(name: impl AsRef<str>) -> String {
    name.as_ref().to_uppercase().replace('-', "_")
}

#[cfg(not(feature = "bindgen"))]
fn bindgen<'a, D, H, X, K, V>(
    out_dir: D,
    _header_file: H,
    _defines: X,
    _add_cflags: Vec<String>,
    _original: bool,
) where
    D: AsRef<Path>,
    H: AsRef<Path>,
    X: IntoIterator<Item = &'a (K, Option<V>)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let target = env::var("TARGET").unwrap();

    if !Path::new("./")
        .join("src")
        .join("bindings")
        .join(format!("{}.rs", target))
        .canonicalize()
        .map(|x| x.exists())
        .unwrap_or(false)
    {
        println!(
            "cargo:warning=rquickjs probably doesn't ship bindings for platform `{}({})`. try the `bindgen` feature instead.",
            target,
            env::var("BUILD_TARGET").unwrap_or("n/a".into())
        );
    }

    let bindings_file = out_dir.as_ref().join("bindings.rs");

    fs::write(
        bindings_file,
        format!(
            r#"macro_rules! bindings_env {{
                ("TARGET") => {{ "{target}" }};
            }}"#
        ),
    )
    .unwrap();
}

#[cfg(feature = "bindgen")]
fn bindgen<'a, D, H, X, K, V>(
    out_dir: D,
    header_file: H,
    defines: X,
    add_cflags: Vec<String>,
    original: bool,
) where
    D: AsRef<Path>,
    H: AsRef<Path>,
    X: IntoIterator<Item = &'a (K, Option<V>)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let out_dir = out_dir.as_ref();
    let header_file = header_file.as_ref();

    let mut cflags = add_cflags;

    //format!("-I{}", out_dir.parent().display()),

    for (name, value) in defines {
        cflags.push(if let Some(value) = value {
            format!("-D{}={}", name.as_ref(), value.as_ref())
        } else {
            format!("-D{}", name.as_ref())
        });
    }

    let mut builder = bindgen_rs::Builder::default()
        .use_core()
        .detect_include_paths(true)
        .clang_arg("-xc")
        .clang_arg("-v")
        .clang_args(cflags)
        .size_t_is_usize(false)
        .header(header_file.display().to_string())
        .allowlist_type("JS.*")
        .allowlist_function("js.*")
        .allowlist_function("JS.*")
        .allowlist_function("__JS.*")
        .allowlist_function("rquickjs_compat_.*")
        .allowlist_var("JS.*")
        .opaque_type("FILE")
        .blocklist_type("FILE")
        .blocklist_function("JS_DumpMemoryUsage");

    // For the original flavor, block the declarations whose signatures diverge
    // from quickjs-ng; the compat shim re-exports normalized versions and the
    // sys crate aliases them back to the quickjs-ng names.
    if original {
        for f in [
            "JS_GetProperty",
            "JS_SetProperty",
            "JS_IsArray",
            "JS_IsError",
            "JS_IsFunction",
            "JS_IsConstructor",
            "JS_NewClassID",
            "JS_HasException",
            "JS_SetConstructorBit",
        ] {
            builder = builder.blocklist_function(f);
        }
    }

    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "wasi" {
        builder = builder.clang_arg("-fvisibility=default");
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    let bindings_file = out_dir.join("bindings.rs");

    bindings
        .write_to_file(&bindings_file)
        .expect("Couldn't write bindings");

    // Special case to support bundled bindings
    if env::var("CARGO_FEATURE_UPDATE_BINDINGS").is_ok() {
        let dest_dir = Path::new("src").join("bindings");
        fs::create_dir_all(&dest_dir).unwrap();

        let dest_file = format!("{}.rs", env::var("TARGET").unwrap());
        fs::copy(&bindings_file, dest_dir.join(dest_file)).unwrap();
    }
}
