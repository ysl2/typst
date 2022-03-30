use lab::Change;
use std::io::Write;
use std::process::Command;
use std::{fs, iter, str};

use criterion::{criterion_group, criterion_main, Criterion};

mod lab;

const XE_POSTFIX: &str = "-xe";
const TYPST_EXT: &str = ".typ";
const TEX_EXT: &str = ".tex";
const TEST_PATH: &str = "applied/";
const TEMP_PATH: &str = "temp/";
const PREFIXES: &[&str] = &["par-s", "del-f"];
const LATEX_WARNING: &str =
    "LaTeX Warning: Label(s) may have changed. Rerun to get cross-references right.";

fn run_echo(file: &str) {
    Command::new("cat").arg(file).output().unwrap();
}

pub fn cal_coma(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-coma-mod.tex", prefix), |b| {
            b.iter(|| {
                file_system_tests(prefix, "coma-mod", FileKind::Latex, |file_name| {
                    run_echo(file_name);
                });
            })
        });
    }
}
pub fn cal_canvas(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-canvas.tex", prefix), |b| {
            b.iter(|| {
                file_system_tests(prefix, "canvas", FileKind::Latex, |file_name| {
                    run_echo(file_name);
                });
            })
        });
    }
}
pub fn cal_shake(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-shake-short.tex", prefix), |b| {
            b.iter(|| {
                file_system_tests(prefix, "shake-short", FileKind::Latex, |file_name| {
                    run_echo(file_name);
                });
            })
        });
    }
}

pub fn pdf_coma(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-coma-mod.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "coma-mod", "pdflatex");
            })
        });
    }
}
pub fn pdf_canvas(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-canvas.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "canvas", "pdflatex");
            })
        });
    }
}
pub fn pdf_shake(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-shake-short.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "shake-short", "pdflatex");
            })
        });
    }
}

pub fn xe_coma(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-coma-mod.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "coma-mod", "xelatex");
            })
        });
    }
}
pub fn xe_canvas(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-canvas.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "canvas", "xelatex");
            })
        });
    }
}
pub fn xe_shake(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-shake-short.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "shake-short", "xelatex");
            })
        });
    }
}

pub fn lua_coma(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-coma-mod.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "coma-mod", "lualatex");
            })
        });
    }
}
pub fn lua_canvas(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-canvas.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "canvas", "lualatex");
            })
        });
    }
}
pub fn lua_shake(c: &mut Criterion) {
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate: {}-shake-short.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "shake-short", "lualatex");
            })
        });
    }
}

// pub fn typst_coma(c: &mut Criterion) {}
// pub fn typst_canvas(c: &mut Criterion) {}
// pub fn typst_shake(c: &mut Criterion) {}

fn tex_test(prefix: &str, name: &str, engine: &str) {
    let kind = if engine.starts_with("pdf") {
        FileKind::Latex
    } else {
        FileKind::Xelatex
    };

    file_system_tests(prefix, name, kind, |file| run_tex(engine, file));
}

fn file_system_tests<F>(prefix: &str, name: &str, kind: FileKind, payload: F)
where
    F: Fn(&str) -> (),
{
    let filename = format!("{}-{}{}", prefix, name, kind.extension());
    let src = fs::read_to_string(format!("{}/{}", TEST_PATH, &filename)).unwrap();
    let lab = lab::Lab::new(&src);
    let mut src = lab.source().to_string();

    let temp_path = format!("{}/{}", TEMP_PATH, &filename);

    for change in iter::once(Change::none()).chain(lab.iter()) {
        fs::create_dir(TEMP_PATH).unwrap();
        src.replace_range(change.range, &change.content);

        let mut file = fs::File::create(&temp_path).unwrap();
        file.write_all(&src.as_bytes()).unwrap();

        payload(&temp_path);

        fs::remove_dir_all(TEMP_PATH).unwrap();
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FileKind {
    Typst,
    Latex,
    Xelatex,
}

impl FileKind {
    fn extension(&self) -> String {
        match self {
            FileKind::Typst => TYPST_EXT.into(),
            FileKind::Latex => TEX_EXT.into(),
            FileKind::Xelatex => format!("{}{}", XE_POSTFIX, TEX_EXT),
        }
    }
}

fn run_tex(engine: &str, file: &str) {
    let mut invoke = Command::new(engine);
    invoke.arg(file);

    let out = invoke.output().unwrap();
    let out = str::from_utf8(&out.stdout).unwrap();

    if out.contains(LATEX_WARNING) {
        invoke.output().unwrap();
    }
}

criterion_group!(calibration, cal_coma, cal_canvas, cal_shake);
criterion_group!(pdflatex, pdf_coma, pdf_canvas, pdf_shake);
criterion_group!(xelatex, xe_coma, xe_canvas, xe_shake);
criterion_group!(lualatex, lua_coma, lua_canvas, lua_shake);
// criterion_group!(typst, typst_coma, typst_canvas, typst_shake);

criterion_main!(pdflatex, xelatex, lualatex, calibration);
