use criterion::measurement::WallTime;
use lab::Change;
use std::io::Write;
use std::process::Command;
use std::{fs, iter, str};

use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};

use typst::export;
use typst::loading::FsLoader;
use typst::Context;

mod lab;

const XE_POSTFIX: &str = "-xe";
const TEX_EXT: &str = ".tex";
const TEST_PATH: &str = "./benches/applied";
const TEMP_PATH: &str = "./benches/temp";
const PREFIXES: &[&str] = &["par-s", "del-f"];
const LATEX_WARNING: &str =
    "LaTeX Warning: Label(s) may have changed. Rerun to get cross-references right.";

fn run_echo(file: &str) {
    Command::new("cat")
        .current_dir(fs::canonicalize("./benches/temp").unwrap())
        .arg(file)
        .output()
        .unwrap();
}

pub fn cal_coma(c: &mut Criterion) {
    let mut c = c.benchmark_group("Calibration Coma");
    c.sample_size(50);
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate:{}-coma-mod.tex", prefix), |b| {
            b.iter(|| {
                file_system_tests(prefix, "coma-mod", FileKind::Latex, |file_name| {
                    run_echo(file_name);
                });
            })
        });
    }
}
pub fn cal_canvas(c: &mut Criterion) {
    let mut c = c.benchmark_group("Calibration Canvas");
    c.sample_size(50);
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate:{}-canvas.tex", prefix), |b| {
            b.iter(|| {
                file_system_tests(prefix, "canvas", FileKind::Latex, |file_name| {
                    run_echo(file_name);
                });
            })
        });
    }
}
pub fn cal_shake(c: &mut Criterion) {
    let mut c = c.benchmark_group("Calibration Shake");
    c.sample_size(50);
    for prefix in PREFIXES {
        c.bench_function(&format!("calibrate:{}-shake-shorter.tex", prefix), |b| {
            b.iter(|| {
                file_system_tests(
                    prefix,
                    "shake-shorter",
                    FileKind::Latex,
                    |file_name| {
                        run_echo(file_name);
                    },
                );
            })
        });
    }
}

pub fn pdf_coma(c: &mut Criterion) {
    let mut c = c.benchmark_group("PDF Coma");
    c.sample_size(50);
    for prefix in PREFIXES {
        c.bench_function(&format!("pdfLaTeX:{}-coma-mod.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "coma-mod", "pdflatex");
            })
        });
    }
}
pub fn pdf_canvas(c: &mut Criterion) {
    let mut c = c.benchmark_group("PDF Canvas");
    c.sample_size(50);
    for prefix in PREFIXES {
        c.bench_function(&format!("pdfLaTeX:{}-canvas.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "canvas", "pdflatex");
            })
        });
    }
}
pub fn pdf_shake(c: &mut Criterion) {
    let mut c = c.benchmark_group("PDF Shake");
    c.sample_size(50);
    for prefix in PREFIXES {
        c.bench_function(&format!("pdfLaTeX:{}-shake-shorter.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "shake-shorter", "pdflatex");
            })
        });
    }
}

pub fn xe_coma(c: &mut Criterion) {
    let mut c = c.benchmark_group("XeLaTeX Coma");
    c.sample_size(50);

    for prefix in PREFIXES {
        c.bench_function(&format!("XeLaTeX:{}-coma-mod.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "coma-mod", "xelatex");
            })
        });
    }
}
pub fn xe_canvas(c: &mut Criterion) {
    let mut c = c.benchmark_group("XeLaTeX Canvas");
    c.sample_size(50);

    for prefix in PREFIXES {
        c.bench_function(&format!("XeLaTeX:{}-canvas.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "canvas", "xelatex");
            })
        });
    }
}
pub fn xe_shake(c: &mut Criterion) {
    let mut c = c.benchmark_group("XeLaTeX Shake");
    c.sample_size(50);

    for prefix in PREFIXES {
        c.bench_function(&format!("XeLaTeX:{}-shake-shorter.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "shake-shorter", "xelatex");
            })
        });
    }
}

pub fn lua_coma(c: &mut Criterion) {
    let mut c = c.benchmark_group("LuaTeX Coma");
    c.sample_size(50);

    for prefix in PREFIXES {
        c.bench_function(&format!("luaLaTeX:{}-coma-mod.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "coma-mod", "lualatex");
            })
        });
    }
}
pub fn lua_canvas(c: &mut Criterion) {
    let mut c = c.benchmark_group("LuaTeX Canvas");
    c.sample_size(50);

    for prefix in PREFIXES {
        c.bench_function(&format!("luaLaTeX:{}-canvas.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "canvas", "lualatex");
            })
        });
    }
}
pub fn lua_shake(c: &mut Criterion) {
    let mut c = c.benchmark_group("LuaTeX Shake");
    c.sample_size(50);

    for prefix in PREFIXES {
        c.bench_function(&format!("luaLaTeX:{}-shake-shorter.tex", prefix), |b| {
            b.iter(|| {
                tex_test(prefix, "shake-shorter", "lualatex");
            })
        });
    }
}

fn context() -> Context {
    let mut loader = FsLoader::new();
    loader.search_path(fs::canonicalize("./fonts").unwrap().as_path());
    loader.search_system();
    Context::new(loader.wrap())
}

fn typst_warmup(c: &mut Criterion) {
    c.bench_function("typst-warmup", |b| {
        b.iter(|| {
            let _ = context();
        })
    });
}

fn typst_bench(prefix: &str, name: &str, c: &mut BenchmarkGroup<WallTime>) {
    let mut ctx = context();

    let filename = format!("{}-{}.typ", prefix, name);
    let temp_path = format!("{}/{}", TEMP_PATH, &filename);

    let src = fs::read_to_string(format!("{}/{}", TEST_PATH, &filename)).unwrap();
    let lab = lab::Lab::new(&src);
    let src = lab.source().to_string();

    fs::create_dir(TEMP_PATH).unwrap();

    c.bench_function(&format!("typst:{}-{}.typ", prefix, name), |b| {
        b.iter(|| {
            let id = ctx.sources.provide(
                fs::canonicalize("./benches/applied/").unwrap().as_path(),
                src.clone(),
            );

            match ctx.typeset(id) {
                // Export the PDF.
                Ok(frames) => {
                    let buffer = export::pdf(&ctx, &frames);
                    fs::write(&temp_path, buffer)
                        .map_err(|_| "failed to write PDF file")
                        .unwrap();
                }

                // Print diagnostics.
                Err(_) => {}
            }

            for change in lab.iter() {
                ctx.sources.edit(id, change.range, &change.content);
                match ctx.typeset(id) {
                    // Export the PDF.
                    Ok(frames) => {
                        let buffer = export::pdf(&ctx, &frames);
                        fs::write(&temp_path, buffer)
                            .map_err(|_| "failed to write PDF file")
                            .unwrap();
                    }

                    Err(_) => {}
                }
            }
        })
    });

    fs::remove_dir_all(TEMP_PATH).unwrap();
}

pub fn typst_coma(c: &mut Criterion) {
    let mut c = c.benchmark_group("Typst Coma");
    c.sample_size(50);
    for prefix in PREFIXES {
        typst_bench(prefix, "coma-mod", &mut c);
    }
}
pub fn typst_canvas(c: &mut Criterion) {
    let mut c = c.benchmark_group("Typst Canvas");
    c.sample_size(50);
    for prefix in PREFIXES {
        typst_bench(prefix, "canvas", &mut c);
    }
}
pub fn typst_shake(c: &mut Criterion) {
    let mut c = c.benchmark_group("Typst Shakespeare");
    c.sample_size(50);
    for prefix in PREFIXES {
        typst_bench(prefix, "shake-shorter", &mut c);
    }
}

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

    fs::create_dir(TEMP_PATH).unwrap();
    for change in iter::once(Change::none()).chain(lab.iter()) {
        src.replace_range(change.range, &change.content);

        let mut file = fs::File::create(&temp_path).unwrap();
        file.write_all(&src.as_bytes()).unwrap();

        payload(&filename);
    }
    fs::remove_dir_all(TEMP_PATH).unwrap();
}

#[derive(Debug, PartialEq, Eq)]
enum FileKind {
    Latex,
    Xelatex,
}

impl FileKind {
    fn extension(&self) -> String {
        match self {
            FileKind::Latex => TEX_EXT.into(),
            FileKind::Xelatex => format!("{}{}", XE_POSTFIX, TEX_EXT),
        }
    }
}

fn run_tex(engine: &str, file: &str) {
    let mut invoke = Command::new(engine);
    invoke.current_dir(fs::canonicalize("./benches/temp").unwrap());
    invoke.arg(file);

    let out = invoke.output().unwrap();
    let out = str::from_utf8(&out.stdout).unwrap_or("");

    if out.contains(LATEX_WARNING) {
        invoke.output().unwrap();
    }
}

criterion_group!(calibration, cal_coma, cal_canvas, cal_shake);
criterion_group!(pdflatex, pdf_coma, pdf_canvas, pdf_shake);
criterion_group!(xelatex, xe_coma, xe_canvas, xe_shake);
criterion_group!(lualatex, lua_coma, lua_canvas, lua_shake);
criterion_group!(typst, typst_warmup, typst_coma, typst_canvas, typst_shake);

criterion_main!(calibration, typst, pdflatex, xelatex, lualatex);
