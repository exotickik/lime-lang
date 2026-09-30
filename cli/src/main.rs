use lime_codegen::Target;
use lime_driver::Compiler;
use std::path::{Path, PathBuf};
use std::process::exit;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        exit(1);
    }
    let cmd = args[0].as_str();
    let rest = &args[1..];
    match cmd {
        "build" => cmd_build(rest),
        "run" => cmd_run(rest),
        "check" => cmd_check(rest),
        "fmt" => cmd_fmt(rest),
        "emit-ir" => cmd_emit_ir(rest),
        "init" => cmd_init(rest),
        "clean" => cmd_clean(rest),
        "version" | "--version" | "-V" => println!("lime {}", VERSION),
        "help" | "--help" | "-h" => print_help(),
        other => {
            eprintln!("error: perintah tidak dikenal '{}'", other);
            eprintln!("jalankan 'lime help' untuk daftar perintah");
            exit(1);
        }
    }
}

struct Common {
    file: PathBuf,
    out: Option<PathBuf>,
    opt_level: u8,
    target: Target,
    passthrough: Vec<String>,
}

fn parse_common(args: &[String], want_file: bool) -> Common {
    let mut file: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut opt_level: u8 = 2;
    let mut target = Target::host();
    let mut passthrough = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-o" | "--output" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    out = Some(PathBuf::from(v));
                } else {
                    fail("opsi '-o' butuh nilai");
                }
            }
            "--target" => {
                i += 1;
                match args.get(i).and_then(|n| Target::by_name(n)) {
                    Some(t) => target = t,
                    None => fail("target tidak dikenal (pilihan: host, linux-x64, windows-x64)"),
                }
            }
            "-O0" => opt_level = 0,
            "-O1" => opt_level = 1,
            "-O2" => opt_level = 2,
            "-O3" => opt_level = 3,
            "--" => {
                passthrough.extend(args[i + 1..].iter().cloned());
                break;
            }
            other => {
                if other.starts_with('-') {
                    fail(&format!("opsi tidak dikenal '{}'", other));
                } else if file.is_none() {
                    file = Some(PathBuf::from(other));
                } else {
                    fail(&format!("argumen tak terduga '{}'", other));
                }
            }
        }
        i += 1;
    }
    let file = match file {
        Some(f) => f,
        None => {
            if want_file {
                fail("butuh file sumber .lime");
            }
            PathBuf::new()
        }
    };
    Common { file, out, opt_level, target, passthrough }
}

fn compiler(c: &Common) -> Compiler {
    Compiler { target: c.target.clone(), opt_level: c.opt_level }
}

fn cmd_build(args: &[String]) {
    let c = parse_common(args, true);
    let comp = compiler(&c);
    match comp.build(&c.file, c.out) {
        Some(exe) => {
            eprintln!("selesai: {}", exe.display());
        }
        None => exit(1),
    }
}

fn cmd_run(args: &[String]) {
    let c = parse_common(args, true);
    let comp = compiler(&c);
    let code = comp.run(&c.file, &c.passthrough);
    exit(code);
}

fn cmd_check(args: &[String]) {
    let c = parse_common(args, true);
    let comp = compiler(&c);
    if !comp.check(&c.file) {
        exit(1);
    }
}

fn cmd_emit_ir(args: &[String]) {
    let c = parse_common(args, true);
    let comp = compiler(&c);
    match comp.emit_ir(&c.file) {
        Some(ir) => print!("{}", ir),
        None => exit(1),
    }
}

fn cmd_fmt(args: &[String]) {
    let mut write = false;
    let mut file: Option<PathBuf> = None;
    for a in args {
        match a.as_str() {
            "--write" | "-w" => write = true,
            other if !other.starts_with('-') => file = Some(PathBuf::from(other)),
            other => fail(&format!("opsi tidak dikenal '{}'", other)),
        }
    }
    let Some(file) = file else { fail("butuh file sumber .lime") };
    let src = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => fail(&format!("tidak bisa membaca '{}': {}", file.display(), e)),
    };
    match lime_fmt::format_source(&file.to_string_lossy(), &src) {
        Ok(formatted) => {
            if write {
                if let Err(e) = std::fs::write(&file, &formatted) {
                    fail(&format!("tidak bisa menulis '{}': {}", file.display(), e));
                }
                eprintln!("terformat: {}", file.display());
            } else {
                print!("{}", formatted);
            }
        }
        Err(errors) => {
            for e in errors {
                eprint!("{}", e);
            }
            exit(1);
        }
    }
}

fn cmd_init(args: &[String]) {
    let name = args.first().cloned().unwrap_or_else(|| "program".to_string());
    let manifest = format!(
        "[paket]\nnama = \"{name}\"\nversi = \"0.1.0\"\n\n[build]\ntarget = \"host\"\n"
    );
    let main_src = format!(
        "program {}\n\nutama {{\n    tampilkan(\"Halo dari Lime!\")\n}}\n",
        capitalize(&name)
    );
    if let Err(e) = std::fs::create_dir_all("src") {
        fail(&format!("tidak bisa membuat direktori 'src': {}", e));
    }
    write_new_file("lime.toml", &manifest);
    write_new_file("src/main.lime", &main_src);
    println!("proyek Lime '{}' dibuat.", name);
    println!("jalankan: lime run src/main.lime");
}

fn cmd_clean(_args: &[String]) {
    let _ = std::fs::remove_dir_all(".lime-build");
    println!("membersihkan artefak build.");
}

fn write_new_file(path: &str, contents: &str) {
    if Path::new(path).exists() {
        eprintln!("lewati: '{}' sudah ada", path);
        return;
    }
    if let Err(e) = std::fs::write(path, contents) {
        fail(&format!("tidak bisa menulis '{}': {}", path, e));
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {}", msg);
    exit(1);
}

fn print_help() {
    println!(
        "lime {VERSION} — toolchain bahasa pemrograman Lime\n\
\n\
PENGGUNAAN:\n\
    lime <perintah> [argumen]\n\
\n\
PERINTAH:\n\
    build <file>     kompilasi ke executable native\n\
    run <file>       kompilasi lalu jalankan\n\
    check <file>     periksa sintaks dan tipe tanpa kompilasi\n\
    fmt <file>       format kode sumber Lime\n\
    emit-ir <file>   cetak LLVM IR yang dihasilkan\n\
    init [nama]        buat proyek Lime baru\n\
    clean              hapus artefak build\n\
    version            tampilkan versi\n\
    help               tampilkan bantuan ini\n\
\n\
OPSI (build/run/check):\n\
    -o, --output <file>   nama executable keluaran\n\
    --target <nama>         host | linux-x64 | windows-x64\n\
    -O0 -O1 -O2 -O3         tingkat optimasi (default -O2)\n\
    -- <args...>            argumen diteruskan ke program (run)\n"
    );
}
