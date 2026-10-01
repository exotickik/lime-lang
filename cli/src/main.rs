mod manifest;

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
        "test" => cmd_test(rest),
        "bench" => cmd_bench(rest),
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

struct Parsed {
    file: Option<PathBuf>,
    out: Option<PathBuf>,
    opt_level: Option<u8>,
    target_name: Option<String>,
    passthrough: Vec<String>,
}

fn parse_args(args: &[String]) -> Parsed {
    let mut file: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut opt_level: Option<u8> = None;
    let mut target_name: Option<String> = None;
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
                match args.get(i) {
                    Some(n) => target_name = Some(n.clone()),
                    None => fail("opsi '--target' butuh nilai"),
                }
            }
            "-O0" => opt_level = Some(0),
            "-O1" => opt_level = Some(1),
            "-O2" => opt_level = Some(2),
            "-O3" => opt_level = Some(3),
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
    Parsed { file, out, opt_level, target_name, passthrough }
}

fn target_by_name(name: &str) -> Target {
    match Target::by_name(name) {
        Some(t) => t,
        None => fail("target tidak dikenal (pilihan: host, linux-x64, windows-x64)"),
    }
}

fn parse_common(args: &[String], _want_file: bool) -> Common {
    let p = parse_args(args);
    match p.file {
        Some(file) => Common {
            file,
            out: p.out,
            opt_level: p.opt_level.unwrap_or(2),
            target: p.target_name.as_deref().map(target_by_name).unwrap_or_else(Target::host),
            passthrough: p.passthrough,
        },
        None => {
            let path = manifest::find().unwrap_or_else(|| {
                fail("butuh file sumber .lime, atau 'lime.toml' di direktori ini (lihat 'lime init')")
            });
            let m = manifest::load(&path).unwrap_or_else(|e| fail(&e));
            let file = m.entry_path();
            if !file.exists() {
                fail(&format!(
                    "entri proyek '{}' tidak ada (atur [build] entri di lime.toml)",
                    file.display()
                ));
            }
            let opt_level = p.opt_level.or(m.optimasi).unwrap_or(2);
            let target = p
                .target_name
                .or(m.target)
                .as_deref()
                .map(target_by_name)
                .unwrap_or_else(Target::host);
            Common { file, out: p.out, opt_level, target, passthrough: p.passthrough }
        }
    }
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

fn cmd_test(args: &[String]) {
    let c = parse_common(args, true);
    let comp = compiler(&c);
    exit(comp.test(&c.file));
}

fn cmd_bench(args: &[String]) {
    let mut iterations: u64 = 1_000_000;
    let mut rest: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--iterasi" || args[i] == "-n" {
            i += 1;
            match args.get(i).and_then(|v| v.parse::<u64>().ok()) {
                Some(v) if v > 0 => iterations = v,
                _ => fail("opsi '--iterasi' butuh angka positif"),
            }
        } else {
            rest.push(args[i].clone());
        }
        i += 1;
    }
    let c = parse_common(&rest, true);
    let comp = compiler(&c);
    exit(comp.bench(&c.file, iterations));
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
    let mut check = false;
    let mut paths: Vec<PathBuf> = Vec::new();
    for a in args {
        match a.as_str() {
            "--write" | "-w" => write = true,
            "--check" | "-c" => check = true,
            other if !other.starts_with('-') => paths.push(PathBuf::from(other)),
            other => fail(&format!("opsi tidak dikenal '{}'", other)),
        }
    }
    if write && check {
        fail("pilih salah satu: '--write' atau '--check'");
    }

    let mut targets: Vec<PathBuf> = Vec::new();
    if paths.is_empty() {
        match manifest::find() {
            Some(p) => {
                let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or_default();
                collect_lime_files(&dir, &mut targets);
            }
            None => fail("butuh file/direktori .lime, atau 'lime.toml' di direktori ini"),
        }
    } else {
        for p in &paths {
            if p.is_dir() {
                collect_lime_files(p, &mut targets);
            } else {
                targets.push(p.clone());
            }
        }
    }
    if targets.is_empty() {
        fail("tidak ada file .lime untuk diformat");
    }
    targets.sort();

    if !write && !check {
        if targets.len() != 1 {
            fail("beberapa file: pakai '--write' untuk menimpa atau '--check' untuk memeriksa");
        }
        let (src, formatted) = fmt_one(&targets[0]);
        let _ = src;
        print!("{}", formatted);
        return;
    }

    let mut needs_format = 0usize;
    for f in &targets {
        let (src, formatted) = fmt_one(f);
        if formatted == src {
            continue;
        }
        if check {
            needs_format += 1;
            println!("perlu format: {}", f.display());
        } else {
            if let Err(e) = std::fs::write(f, &formatted) {
                fail(&format!("tidak bisa menulis '{}': {}", f.display(), e));
            }
            eprintln!("terformat: {}", f.display());
        }
    }
    if check && needs_format > 0 {
        eprintln!("{} file perlu diformat", needs_format);
        exit(1);
    }
}

fn fmt_one(file: &Path) -> (String, String) {
    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => fail(&format!("tidak bisa membaca '{}': {}", file.display(), e)),
    };
    match lime_fmt::format_source(&file.to_string_lossy(), &src) {
        Ok(formatted) => (src, formatted),
        Err(errors) => {
            for e in errors {
                eprint!("{}", e);
            }
            exit(1);
        }
    }
}

fn collect_lime_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => fail(&format!("tidak bisa membaca direktori '{}': {}", dir.display(), e)),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .map(|n| n == "target" || n == ".lime-build" || n == ".git")
                .unwrap_or(false);
            if !skip {
                collect_lime_files(&path, out);
            }
        } else if path.extension().map(|e| e == "lime").unwrap_or(false) {
            out.push(path);
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
    test <file>      jalankan fungsi uji ('uji_*') di file\n\
    bench <file>     ukur fungsi tolok ukur ('tolok_*'); --iterasi N\n\
    fmt [path...]    format kode (--write menimpa, --check periksa; dir/proyek)\n\
    emit-ir <file>   cetak LLVM IR yang dihasilkan\n\
    init [nama]        buat proyek Lime baru\n\
    clean              hapus artefak build\n\
    version            tampilkan versi\n\
    help               tampilkan bantuan ini\n\
\n\
Tanpa argumen <file>, perintah build/run/check/test/bench memakai 'lime.toml'\n\
di direktori ini (atau induknya). Lihat 'lime init'.\n\
\n\
OPSI (build/run/check):\n\
    -o, --output <file>   nama executable keluaran\n\
    --target <nama>         host | linux-x64 | windows-x64\n\
    -O0 -O1 -O2 -O3         tingkat optimasi (default -O2)\n\
    -- <args...>            argumen diteruskan ke program (run)\n"
    );
}
