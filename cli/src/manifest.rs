use std::path::{Path, PathBuf};

pub struct Manifest {
    pub nama: Option<String>,
    pub versi: Option<String>,
    pub entri: Option<String>,
    pub target: Option<String>,
    pub optimasi: Option<u8>,
    pub dir: PathBuf,
}

impl Manifest {
    pub fn entry_path(&self) -> PathBuf {
        let rel = self.entri.clone().unwrap_or_else(|| "src/main.exo".to_string());
        self.dir.join(rel)
    }
}

pub fn find() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("exotickik.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn load(path: &Path) -> Result<Manifest, String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("tidak bisa membaca '{}': {}", path.display(), e))?;
    let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut m = Manifest {
        nama: None,
        versi: None,
        entri: None,
        target: None,
        optimasi: None,
        dir,
    };
    let mut section = String::new();
    for (i, raw) in src.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            let name = rest.strip_suffix(']').ok_or_else(|| {
                format!("{}:{}: judul seksi tidak ditutup ']'", path.display(), i + 1)
            })?;
            section = name.trim().to_string();
            continue;
        }
        let eq = line.find('=').ok_or_else(|| {
            format!("{}:{}: baris bukan 'kunci = nilai'", path.display(), i + 1)
        })?;
        let key = line[..eq].trim();
        let value = unquote(line[eq + 1..].trim());
        match (section.as_str(), key) {
            ("paket", "nama") => m.nama = Some(value),
            ("paket", "versi") => m.versi = Some(value),
            ("build", "entri") => m.entri = Some(value),
            ("build", "target") => m.target = Some(value),
            ("build", "optimasi") => {
                let n = value.parse::<u8>().map_err(|_| {
                    format!("{}:{}: 'optimasi' harus 0..3", path.display(), i + 1)
                })?;
                if n > 3 {
                    return Err(format!("{}:{}: 'optimasi' harus 0..3", path.display(), i + 1));
                }
                m.optimasi = Some(n);
            }
            _ => {}
        }
    }
    Ok(m)
}

fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '#' if !in_str => return &line[..i],
            _ => {}
        }
    }
    line
}

fn unquote(s: &str) -> String {
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"' {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_tagged(tag: &str, src: &str) -> Manifest {
        let dir = std::env::temp_dir().join(format!("exotickik_mani_{}_{}", tag, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("exotickik.toml");
        std::fs::write(&path, src).unwrap();
        load(&path).unwrap()
    }

    #[test]
    fn reads_fields_and_sections() {
        let m = parse_tagged(
            "fields",
            "[paket]\nnama = \"halo\"\nversi = \"0.1.0\"\n\n[build]\nentri = \"kode/a.exo\"\noptimasi = 1\ntarget = \"host\"\n",
        );
        assert_eq!(m.nama.as_deref(), Some("halo"));
        assert_eq!(m.entri.as_deref(), Some("kode/a.exo"));
        assert_eq!(m.optimasi, Some(1));
        assert_eq!(m.target.as_deref(), Some("host"));
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let m = parse_tagged("comments", "# komentar\n\n[paket]\nnama = \"x\"  # trailing\n");
        assert_eq!(m.nama.as_deref(), Some("x"));
    }

    #[test]
    fn hash_inside_string_is_kept() {
        let m = parse_tagged("hash", "[paket]\nnama = \"a#b\"\n");
        assert_eq!(m.nama.as_deref(), Some("a#b"));
    }

    #[test]
    fn default_entry_when_unset() {
        let m = parse_tagged("defentry", "[paket]\nnama = \"x\"\n");
        assert!(m.entry_path().ends_with("src/main.exo"));
    }

    #[test]
    fn rejects_bad_optimasi() {
        let dir = std::env::temp_dir().join(format!("exotickik_mani_bad_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("exotickik.toml");
        std::fs::write(&path, "[build]\noptimasi = 9\n").unwrap();
        assert!(load(&path).is_err());
    }
}
