//! Composable layouts.

mod abs;
mod align;
mod angle;
mod axes;
mod columns;
mod container;
mod corners;
mod dir;
mod em;
mod flow;
mod fr;
mod fragment;
mod frame;
mod grid;
mod hide;
mod inline;
#[path = "layout.rs"]
mod layout_;
mod length;
#[path = "measure.rs"]
mod measure_;
mod pad;
mod page;
mod place;
mod point;
mod ratio;
mod regions;
mod rel;
mod repeat;
mod sides;
mod size;
mod spacing;
mod stack;
mod transform;

pub use self::abs::*;
pub use self::align::*;
pub use self::angle::*;
pub use self::axes::*;
pub use self::columns::*;
pub use self::container::*;
pub use self::corners::*;
pub use self::dir::*;
pub use self::em::*;
pub use self::flow::*;
pub use self::fr::*;
pub use self::fragment::*;
pub use self::frame::*;
pub use self::grid::*;
pub use self::hide::*;
pub use self::layout_::*;
pub use self::length::*;
pub use self::measure_::*;
pub use self::pad::*;
pub use self::page::*;
pub use self::place::*;
pub use self::point::*;
pub use self::ratio::*;
pub use self::regions::Regions;
pub use self::rel::*;
pub use self::repeat::*;
pub use self::sides::*;
pub use self::size::*;
pub use self::spacing::*;
pub use self::stack::*;
pub use self::transform::*;

pub(crate) use self::inline::*;

use comemo::{Tracked, TrackedMut};

use crate::diag::{bail, SourceResult};
use crate::engine::{Engine, Route};
use crate::eval::Tracer;
use crate::foundations::{category, Category, Content, Scope};
use crate::introspection::Context;
use crate::introspection::Introspector;
use crate::model::Document;
use crate::realize::{realize_block, realize_root, Scratch};
use crate::World;

/// Arranging elements on the page in different ways.
///
/// By combining layout functions, you can create complex and automatic layouts.
#[category]
pub static LAYOUT: Category;

/// Hook up all `layout` definitions.
pub fn define(global: &mut Scope) {
    global.category(LAYOUT);
    global.define_type::<Length>();
    global.define_type::<Angle>();
    global.define_type::<Ratio>();
    global.define_type::<Rel<Length>>();
    global.define_type::<Fr>();
    global.define_type::<Dir>();
    global.define_type::<Align>();
    global.define_elem::<PageElem>();
    global.define_elem::<PagebreakElem>();
    global.define_elem::<VElem>();
    global.define_elem::<HElem>();
    global.define_elem::<BoxElem>();
    global.define_elem::<BlockElem>();
    global.define_elem::<StackElem>();
    global.define_elem::<GridElem>();
    global.define_elem::<ColumnsElem>();
    global.define_elem::<ColbreakElem>();
    global.define_elem::<PlaceElem>();
    global.define_elem::<AlignElem>();
    global.define_elem::<PadElem>();
    global.define_elem::<RepeatElem>();
    global.define_elem::<MoveElem>();
    global.define_elem::<ScaleElem>();
    global.define_elem::<RotateElem>();
    global.define_elem::<HideElem>();
    global.define_func::<measure>();
    global.define_func::<layout>();
}

/// Root-level layout.
pub trait LayoutRoot {
    /// Layout into one frame per page.
    fn layout_root(
        &self,
        engine: &mut Engine,
        context: Context,
    ) -> SourceResult<Document>;
}

/// Layout into regions.
pub trait Layout {
    /// Layout into one frame per region.
    fn layout(
        &self,
        engine: &mut Engine,
        context: Context,
        regions: Regions,
    ) -> SourceResult<Fragment>;
}

impl LayoutRoot for Content {
    #[tracing::instrument(name = "Content::layout_root", skip_all)]
    fn layout_root(
        &self,
        engine: &mut Engine,
        context: Context,
    ) -> SourceResult<Document> {
        #[comemo::memoize]
        fn cached(
            content: &Content,
            world: Tracked<dyn World + '_>,
            introspector: Tracked<Introspector>,
            route: Tracked<Route>,
            tracer: TrackedMut<Tracer>,
            context: Context,
        ) -> SourceResult<Document> {
            let mut engine = Engine {
                world,
                introspector,
                route: Route::extend(route).unnested(),
                tracer,
            };
            let scratch = Scratch::default();
            let (realized, styles) =
                realize_root(&mut engine, &scratch, content, context.variant(1))?;
            realized
                .with::<dyn LayoutRoot>()
                .unwrap()
                .layout_root(&mut engine, Context { styles, ..context.variant(2) })
        }

        tracing::info!("Starting layout");
        cached(
            self,
            engine.world,
            engine.introspector,
            engine.route.track(),
            TrackedMut::reborrow_mut(&mut engine.tracer),
            context,
        )
    }
}

impl Layout for Content {
    #[tracing::instrument(name = "Content::layout", skip_all)]
    fn layout(
        &self,
        engine: &mut Engine,
        context: Context,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        #[allow(clippy::too_many_arguments)]
        #[comemo::memoize]
        fn cached(
            content: &Content,
            world: Tracked<dyn World + '_>,
            introspector: Tracked<Introspector>,
            route: Tracked<Route>,
            tracer: TrackedMut<Tracer>,
            context: Context,
            regions: Regions,
        ) -> SourceResult<Fragment> {
            let mut engine = Engine {
                world,
                introspector,
                route: Route::extend(route),
                tracer,
            };

            if !engine.route.within(Route::MAX_LAYOUT_DEPTH) {
                bail!(
                    content.span(), "maximum layout depth exceeded";
                    hint: "try to reduce the amount of nesting in your layout",
                );
            }

            let scratch = Scratch::default();
            let (realized, styles) =
                realize_block(&mut engine, &scratch, content, context.variant(1))?;
            realized.with::<dyn Layout>().unwrap().layout(
                &mut engine,
                Context { styles, ..context.variant(2) },
                regions,
            )
        }

        tracing::info!("Layouting `Content`");

        cached(
            self,
            engine.world,
            engine.introspector,
            engine.route.track(),
            TrackedMut::reborrow_mut(&mut engine.tracer),
            context,
            regions,
        )
    }
}
