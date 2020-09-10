use std::mem;

use super::*;
use crate::geom::shape::Shape;
use crate::geom::TranslateScale;

pub struct StackLayouter {
    opts: StackOptions,
    curr: Option<Current>,
    areas: Areas,
    done: Vec<Layout>,
}

struct Current {
    area: Area,
    layout: Layout,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct StackOptions {
    pub dir: Dir,
}

impl StackLayouter {
    pub fn new(mut areas: Areas, opts: StackOptions) -> Self {
        Self {
            opts,
            curr: areas.next().map(Current::new),
            areas,
            done: vec![],
        }
    }

    pub fn finish(mut self) -> Vec<Layout> {
        self.finish_area();
        self.done
    }
}

impl Layouter for StackLayouter {
    fn remaining(&self) -> (Option<&Area>, &Areas) {
        (self.curr.as_ref().map(|c| &c.area), &self.areas)
    }

    fn spacing(&mut self, axis: SpecAxis, amount: f64) {
        if axis == self.opts.dir.axis() {
            let curr = try_or!(self.curr.as_mut(), return);
            curr.shrink_by_amount(self.opts.dir, amount);
        }
    }

    fn layout_movable(&mut self, align: GenAlign, layout: Layout) {
        if let Some((id, pos)) = self.place(layout.dim, align) {
            self.skip_to_area(id);
            let curr = try_or!(self.curr.as_mut(), return);
            curr.shrink_by_placed(self.opts.dir, pos, layout.dim);
            curr.layout.push_layout(pos, layout);
        } else {
            println!("warn: failed to fit object into any area");
        }
    }

    fn layout_immovable(&mut self, pos: Point, collider: Collider, layout: Layout) {
        let curr = try_or!(self.curr.as_mut(), return);
        curr.shrink_by_collider(pos, collider, &layout);
        curr.layout.push_layout(pos, layout);
    }
}

impl StackLayouter {
    fn place(&self, dim: Dim, align: GenAlign) -> Option<(usize, Point)> {
        for (i, area) in
            self.curr.iter().map(|c| &c.area).chain(self.areas.iter()).enumerate()
        {
            assert_eq!(align, GenAlign::Start);
            let side = self.opts.dir.start();
            if let Some(pos) = area.place(dim, side) {
                return Some((i, pos));
            }
        }

        None
    }

    fn skip_to_area(&mut self, i: usize) {
        for _ in 0 .. i {
            self.finish_area();
        }
    }

    fn finish_area(&mut self) {
        let next = self.areas.next().map(Current::new);
        let curr = mem::replace(&mut self.curr, next);
        if let Some(Current { layout, .. }) = curr {
            self.done.push(layout);
        }
    }
}

impl Current {
    fn new(area: Area) -> Self {
        // TODO: Allow configurable baseline.
        let dim = Dim::new(area.size.width, 0.0, area.size.height);
        Self { area, layout: Layout::new(dim) }
    }

    fn shrink_by_amount(&mut self, dir: Dir, amount: f64) {
        self.area.shrink_by(amount, dir.start());
    }

    fn shrink_by_placed(&mut self, dir: Dir, pos: Point, dim: Dim) {
        let to = match dir {
            Dir::LTR => pos.x + dim.width,
            Dir::RTL => pos.x,
            Dir::TTB => pos.y + dim.depth,
            Dir::BTT => pos.y - dim.height,
        };

        self.area.shrink_to(to, dir.start());
    }

    fn shrink_by_collider(&mut self, pos: Point, collider: Collider, layout: &Layout) {
        // Tolerance is ignored for rectangles.
        const RECT_EPS: f64 = f64::INFINITY;

        let path: BezPath = match collider {
            Collider::None => return,
            Collider::Tight => todo!("tight collider"),
            Collider::Bounds => layout.dim.to_rect().to_bez_path(RECT_EPS).collect(),
            Collider::Row => {
                let mut rect = layout.dim.to_rect();
                rect.x0 = f64::NEG_INFINITY;
                rect.x1 = f64::INFINITY;
                rect.to_bez_path(RECT_EPS).collect()
            }
            Collider::Column => {
                let mut rect = layout.dim.to_rect();
                rect.y0 = f64::NEG_INFINITY;
                rect.y1 = f64::INFINITY;
                rect.to_bez_path(RECT_EPS).collect()
            }
        };

        let ts = TranslateScale::translate(pos.to_vec2());
        self.area.subtract(&(ts * path));
    }
}
