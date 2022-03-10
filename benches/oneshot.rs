use std::path::Path;
use std::sync::Arc;

use iai::{black_box, main, Iai};

use typst::loading::MemLoader;
use typst::parse::{parse, Scanner, TokenMode, Tokens};
use typst::source::SourceId;
use typst::{Context, Vm};

mod lab;

const SRC: &str = include_str!("bench_paper.typ");
const FONT: &[u8] = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");
const LBR: &[u8] = include_bytes!("../fonts/LinBiolinum_Rah.ttf");
const LBB: &[u8] = include_bytes!("../fonts/LinBiolinum_RIah.ttf");
const LBI: &[u8] = include_bytes!("../fonts/LinBiolinum_RBah.ttf");
const LLR: &[u8] = include_bytes!("../fonts/LinLibertine_Rah.ttf");
const LLB: &[u8] = include_bytes!("../fonts/LinLibertine_RIah.ttf");
const LLI: &[u8] = include_bytes!("../fonts/LinLibertine_RBah.ttf");

fn loader() -> Arc<MemLoader> {
    let mut loader = MemLoader::new().with(Path::new("font.ttf"), FONT);
    loader.insert("LinBiolinum_regular.ttf", LBR);
    loader.insert("LinBiolinum_italic.ttf", LBI);
    loader.insert("LinBiolinum_bold.ttf", LBB);
    loader.insert("LinLibertine_regular.ttf", LLR);
    loader.insert("LinLibertine_italic.ttf", LLI);
    loader.insert("LinLibertine_bold.ttf", LLB);
    loader.wrap()
}

fn context() -> (Context, SourceId) {
    let mut ctx = Context::new(loader());
    let id = ctx.sources.provide(Path::new("src.typ"), SRC.to_string());
    (ctx, id)
}

main!(
    bench_decode,
    bench_scan,
    bench_tokenize,
    bench_parse,
    bench_edit,
    bench_eval,
    bench_layout,
    bench_cal,
    bench_lab,
    bench_full,
    bench_highlight,
    bench_byte_to_utf16,
    bench_render,
);

fn bench_decode(iai: &mut Iai) {
    iai.run(|| {
        // We don't use chars().count() because that has a special
        // superfast implementation.
        let mut count = 0;
        let mut chars = black_box(SRC).chars();
        while let Some(_) = chars.next() {
            count += 1;
        }
        count
    })
}

fn bench_scan(iai: &mut Iai) {
    iai.run(|| {
        let mut count = 0;
        let mut scanner = Scanner::new(black_box(SRC));
        while let Some(_) = scanner.eat() {
            count += 1;
        }
        count
    })
}

fn bench_tokenize(iai: &mut Iai) {
    iai.run(|| Tokens::new(black_box(SRC), black_box(TokenMode::Markup)).count());
}

fn bench_parse(iai: &mut Iai) {
    iai.run(|| parse(SRC));
}

fn bench_edit(iai: &mut Iai) {
    let (mut ctx, id) = context();
    iai.run(|| black_box(ctx.sources.edit(id, 57 .. 58, "_Uhr_")));
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

fn bench_highlight(iai: &mut Iai) {
    let (ctx, id) = context();
    let source = ctx.sources.get(id);
    iai.run(|| source.highlight(0 .. source.len_bytes(), |_, _| {}));
}

fn bench_byte_to_utf16(iai: &mut Iai) {
    let (ctx, id) = context();
    let source = ctx.sources.get(id);
    let mut ranges = vec![];
    source.highlight(0 .. source.len_bytes(), |range, _| ranges.push(range));
    iai.run(|| {
        ranges
            .iter()
            .map(|range| source.byte_to_utf16(range.start)
                .. source.byte_to_utf16(range.end))
            .collect::<Vec<_>>()
    });
}

fn bench_cal(iai: &mut Iai) {
    let lab = lab::Lab::new(SRC);
    iai.run(|| {
        for _ in lab.iter() {
            continue;
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

    iai.run(|| {
        for change in lab.iter() {
            vm.sources.edit(id, change.range, &change.content);
            let module = vm.evaluate(id).unwrap();
            module.template.layout_pages(&mut vm).unwrap();
        }
    });
}

fn bench_full(iai: &mut Iai) {
    iai.run(|| {
        let (mut ctx, id) = context();
        ctx.typeset(id).unwrap();
    })
}

fn bench_render(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let frames = ctx.typeset(id).unwrap();
    iai.run(|| typst::export::render(&mut ctx, &frames[0], 1.0))
}
