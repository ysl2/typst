use std::fmt::{self, Debug, Formatter};

use super::prelude::*;
use super::{AlignNode, SpacingKind, SpacingNode};

/// Defines how to size a grid cell along an axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ColumnSizing {
    /// A length stated in absolute values and/or relative to the parent's size.
    Linear(Linear),
    /// A length that is the fraction of the remaining free space in the parent.
    Fractional(Fractional),
}

/// A node that separates a region into multiple columns.
#[derive(Debug, Hash)]
pub struct ColumnsNode {
    /// The columns' direction.
    pub dir: Dir,
    /// The size of each column. There must be at least one column.554
    pub columns: Vec<ColumnSizing>,
    /// The size of the gutter space between each column. If there are less
    /// elements here than `columns.len() - 1` then the last element is
    /// repeated, if there are no elements, the default will be `8pt`s.
    pub gutter: Vec<ColumnSizing>,
    /// The child to be layouted into the columns. Most likely, this should be a
    /// flow or stack node.
    pub child: PackedNode,
}

impl Layout for ColumnsNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // All gutters in the document. (Can be different because the relative
        // component is calculated seperately for each region.)
        let mut gutters = vec![];
        // Sizes of all columns resulting from `region.current` and `region.backlog`
        let mut sizes = vec![];

        for (current, base) in std::iter::once((regions.current, regions.base))
            .chain(regions.backlog.clone().into_iter().map(|s| (s, s)))
        {
            let (columns, local_gutter, main) = self.measure(current, base);
            sizes.extend(columns.map(|col| Gen::new(col, main).to_spec(self.dir.axis())));
            gutters.extend(local_gutter);
        }

        // As I said before, there should be at least one column.
        let first = sizes.remove(0);
        let mut regions = Regions::one(first, first, regions.expand);
        regions.backlog = sizes.into_iter();

        // We have to treat the last region separately.
        let (last_columns, last_gutter, last_main) = match regions.last {
            Some(last) => {
                let (a, b, c) = self.measure(last, last);
                (Some(a), Some(b), Some(c))
            }
            None => (None, None, None),
        };

        // We now have the problem that the `last` item in the region is
        // potentially disintegrating into multiple items that have to be cycled
        // indefinitely which the current region model does not allow for.
        //
        // A potential remedy would be to change the type of last into `Box<dyn
        // IntoIterator<Item = Spec<Length>>>` which either has no elements or
        // is infinite.

        todo!()
    }
}

impl ColumnsNode {
    /// Return the length of each column, the gutter in between, and the shared
    /// height of all of them.
    fn measure<'a>(
        &'a self,
        current: Spec<Length>,
        base: Spec<Length>,
    ) -> (
        impl Iterator<Item = Length> + 'a,
        impl Iterator<Item = Length> + 'a,
        Length,
    ) {
        let mut total_fr = Fractional::zero();
        let remaining = current.get(self.dir.axis())
            - self
                .columns
                .iter()
                .chain(self.gutter.iter())
                .filter_map(|size| match size {
                    ColumnSizing::Linear(l) => Some(l.resolve(base.get(self.dir.axis()))),
                    ColumnSizing::Fractional(fr) => {
                        total_fr += *fr;
                        None
                    }
                })
                .sum::<Length>();

        let columns = self.columns.iter().copied().map(move |size| {
            match size {
                ColumnSizing::Linear(l) => l.resolve(base.get(self.dir.axis())),
                ColumnSizing::Fractional(fr) => fr.resolve(total_fr, remaining),
            }
        });

        let default_gutter = self
            .gutter
            .last()
            .copied()
            .unwrap_or(ColumnSizing::Linear(Length::pt(8.0).into()));

        let gutter = self
            .gutter
            .iter()
            .copied()
            .chain(std::iter::repeat(default_gutter))
            .take(columns.len() - 1)
            .map(move |size| {
                match size {
                    ColumnSizing::Linear(l) => l.resolve(base.get(self.dir.axis())),
                    ColumnSizing::Fractional(fr) => fr.resolve(total_fr, remaining),
                }
            });

        let main = current.get(self.dir.axis().other());

        (columns, gutter, main)
    }
}

castable! {
    Vec<ColumnSizing>,
    Expected: "integer or (linear, fractional, or array thereof)",
    Value::Length(v) => vec![ColumnSizing::Linear(v.into())],
    Value::Relative(v) => vec![ColumnSizing::Linear(v.into())],
    Value::Linear(v) => vec![ColumnSizing::Linear(v)],
    Value::Fractional(v) => vec![ColumnSizing::Fractional(v)],
    Value::Int(count) => vec![ColumnSizing::Fractional(Fractional::one()); count.max(0) as usize],
    Value::Array(values) => values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect(),
}

castable! {
    ColumnSizing,
    Expected: "linear, or fractional",
    Value::Length(v) => Self::Linear(v.into()),
    Value::Relative(v) => Self::Linear(v.into()),
    Value::Linear(v) => Self::Linear(v),
    Value::Fractional(v) => Self::Fractional(v),
}
