use std::path::Path;
use std::sync::Arc;

use iai::{black_box, main, Iai};

use typst::loading::MemLoader;
use typst::parse::{parse, Scanner, TokenMode, Tokens};
use typst::source::SourceId;
use typst::{Context, Vm};

mod lab;

const SRC: &str = include_str!("applied/del-f-shake-shorter.typ");
// const FONT: &[u8] = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");
const IBM_R: &[u8] = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");
const IBM_B: &[u8] = include_bytes!("../fonts/IBMPlexSans-Bold.ttf");
const IBM_I: &[u8] = include_bytes!("../fonts/IBMPlexSans-Italic.ttf");
const BUE_R: &[u8] = include_bytes!("../fonts/Buenard-Regular.ttf");
const BUE_F: &[u8] = include_bytes!("../fonts/Buenard-Bold.ttf");
// const LBR: &[u8] = include_bytes!("../fonts/LinBiolinum_Rah.ttf");
// const LBB: &[u8] = include_bytes!("../fonts/LinBiolinum_RIah.ttf");
// const LBI: &[u8] = include_bytes!("../fonts/LinBiolinum_RBah.ttf");
// const LLR: &[u8] = include_bytes!("../fonts/LinLibertine_Rah.ttf");
// const LLB: &[u8] = include_bytes!("../fonts/LinLibertine_RIah.ttf");
// const LLI: &[u8] = include_bytes!("../fonts/LinLibertine_RBah.ttf");

const IMG_1: &[u8] = include_bytes!("assets/block-latex.png");
const IMG_2: &[u8] = include_bytes!("assets/block-word.png");
const IMG_3: &[u8] = include_bytes!("assets/gradient@2x-t.png");
const IMG_4: &[u8] = include_bytes!("assets/graph.png");
const IMG_5: &[u8] = include_bytes!("assets/rund-l.png");
const IMG_6: &[u8] = include_bytes!("assets/rund-m.png");
const IMG_7: &[u8] = include_bytes!("assets/rund-u.png");
const IMG_8: &[u8] = include_bytes!("assets/venn.svg");

fn loader() -> Arc<MemLoader> {
    let mut loader = MemLoader::new().with(Path::new("IBMPlexSans-Regular.ttf"), IBM_R);
    loader.insert("IBMPlexSans-Bold.ttf", IBM_B);
    loader.insert("IBMPlexSans-Italic.ttf", IBM_I);
    loader.insert("Buenard-Regular.ttf", BUE_R);
    loader.insert("Buenard-Bold.ttf", BUE_F);

    loader.insert("block-latex.png", IMG_1);
    loader.insert("block-word.png", IMG_2);
    loader.insert("gradient@2x-t.png", IMG_3);
    loader.insert("graph.png", IMG_4);
    loader.insert("rund-l.png", IMG_5);
    loader.insert("rund-m.png", IMG_6);
    loader.insert("rund-u.png", IMG_7);
    loader.insert("venn.svg", IMG_8);

    // loader.insert("LinBiolinum_regular.ttf", LBR);
    // loader.insert("LinBiolinum_italic.ttf", LBI);
    // loader.insert("LinBiolinum_bold.ttf", LBB);
    // loader.insert("LinLibertine_regular.ttf", LLR);
    // loader.insert("LinLibertine_italic.ttf", LLI);
    // loader.insert("LinLibertine_bold.ttf", LLB);
    loader.wrap()
}

fn context() -> (Context, SourceId) {
    let mut ctx = Context::new(loader());
    let id = ctx.sources.provide(Path::new("src.typ"), SRC.to_string());
    (ctx, id)
}

main!(
    // bench_decode,
    // bench_scan,
    // bench_tokenize,
    bench_setup,
    bench_parse,
    bench_eval,
    bench_layout,
    bench_full,
    bench_cal,
    bench_edit,
    bench_lab_eval,
    bench_lab,
    // bench_highlight,
    // bench_byte_to_utf16,
    // bench_render,
);

// fn bench_decode(iai: &mut Iai) {
//     iai.run(|| {
//         // We don't use chars().count() because that has a special
//         // superfast implementation.
//         let mut count = 0;
//         let mut chars = black_box(SRC).chars();
//         while let Some(_) = chars.next() {
//             count += 1;
//         }
//         count
//     })
// }

// fn bench_scan(iai: &mut Iai) {
//     iai.run(|| {
//         let mut count = 0;
//         let mut scanner = Scanner::new(black_box(SRC));
//         while let Some(_) = scanner.eat() {
//             count += 1;
//         }
//         count
//     })
// }

// fn bench_tokenize(iai: &mut Iai) {
//     iai.run(|| Tokens::new(black_box(SRC), black_box(TokenMode::Markup)).count());
// }

fn bench_setup(iai: &mut Iai) {
    iai.run(|| context());
}

fn bench_parse(iai: &mut Iai) {
    iai.run(|| parse(SRC));
}

fn bench_edit(iai: &mut Iai) {
    let lab = lab::Lab::new(SRC);

    let mut ctx = Context::new(loader());
    let id = ctx.sources.provide(Path::new("src.typ"), lab.source().to_string());

    iai.run(|| {
        for change in lab.iter() {
            ctx.sources.edit(id, change.range, &change.content);
        }
    });
}

fn bench_eval(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let mut vm = Vm::new(&mut ctx);
    iai.run(|| vm.evaluate(id).unwrap());
}

fn bench_layout(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let mut vm = Vm::new(&mut ctx);
    let module = vm.evaluate(id).unwrap();
    iai.run(|| module.template.layout_pages(&mut vm));
}

// fn bench_highlight(iai: &mut Iai) {
//     let (ctx, id) = context();
//     let source = ctx.sources.get(id);
//     iai.run(|| source.highlight(0 .. source.len_bytes(), |_, _| {}));
// }

// fn bench_byte_to_utf16(iai: &mut Iai) {
//     let (ctx, id) = context();
//     let source = ctx.sources.get(id);
//     let mut ranges = vec![];
//     source.highlight(0 .. source.len_bytes(), |range, _| ranges.push(range));
//     iai.run(|| {
//         ranges
//             .iter()
//             .map(|range| source.byte_to_utf16(range.start)
//                 .. source.byte_to_utf16(range.end))
//             .collect::<Vec<_>>()
//     });
// }

fn bench_cal(iai: &mut Iai) {
    let lab = lab::Lab::new(SRC);
    iai.run(|| {
        for _ in lab.iter() {
            continue;
        }
    });
}

fn bench_lab_eval(iai: &mut Iai) {
    let lab = lab::Lab::new(SRC);

    let mut ctx = Context::new(loader());
    let id = ctx.sources.provide(Path::new("src.typ"), lab.source().to_string());
    let mut vm = Vm::new(&mut ctx);
    if vm.evaluate(id).is_err() {
        println!("!");
    }

    iai.run(|| {
        for change in lab.iter() {
            vm.modules.clear();
            vm.sources.edit(id, change.range, &change.content);
            if vm.evaluate(id).is_err() {
                println!("!");
            }
        }
    });
}

fn bench_lab(iai: &mut Iai) {
    let lab = lab::Lab::new(SRC);

    let mut ctx = Context::new(loader());
    let id = ctx.sources.provide(Path::new("src.typ"), lab.source().to_string());
    let mut vm = Vm::new(&mut ctx);
    let module = vm.evaluate(id).unwrap();
    module.template.layout_pages(&mut vm).unwrap();
    vm.modules.clear();

    iai.run(|| {
        for change in lab.iter() {
            vm.modules.clear();
            vm.sources.edit(id, change.range, &change.content);
            if let Ok(module) = vm.evaluate(id) {
                if module.template.layout_pages(&mut vm).is_err() {
                    println!("!");
                }
            }
        }
    });
}

fn bench_full(iai: &mut Iai) {
    iai.run(|| {
        let (mut ctx, id) = context();
        ctx.typeset(id).unwrap();
    })
}

// fn bench_render(iai: &mut Iai) {
//     let (mut ctx, id) = context();
//     let frames = ctx.typeset(id).unwrap();
//     iai.run(|| typst::export::render(&mut ctx, &frames[0], 1.0))
// }
