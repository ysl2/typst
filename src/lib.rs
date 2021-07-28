//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [syntax
//!   tree]. The structures describing the tree can be found in the [syntax]
//!   module.
//! - **Evaluation:** The next step is to [evaluate] the syntax tree. This
//!   computes the value of each node in the document and produces a [module].
//! - **Execution:** Now, we can [execute] the parsed and evaluated module.
//!   This produces a [layout tree], a high-level, fully styled representation
//!   of the document. The nodes of this tree are self-contained and
//!   order-independent and thus much better suited for layouting than the
//!   syntax tree.
//! - **Layouting:** Next, the tree is [layouted] into a portable version of the
//!   typeset document. The output of this is a collection of [`Frame`]s (one
//!   per page), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Currently, the only supported output format is [PDF].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [syntax tree]: syntax::SyntaxTree
//! [evaluate]: eval::eval
//! [module]: eval::Module
//! [execute]: exec::exec
//! [layout tree]: layout::LayoutTree
//! [layouted]: layout::layout
//! [PDF]: export::pdf

#[macro_use]
pub mod diag;
#[macro_use]
pub mod eval;
pub mod color;
pub mod eco;
pub mod export;
pub mod font;
pub mod geom;
pub mod image;
pub mod layout;
pub mod library;
pub mod loading;
pub mod paper;
pub mod parse;
pub mod pretty;
pub mod syntax;
pub mod util;

use std::rc::Rc;

use crate::diag::Pass;
use crate::eval::{ModuleCache, Scope, Defaults};
use crate::font::FontCache;
use crate::image::ImageCache;
use crate::layout::Frame;
#[cfg(feature = "layout-cache")]
use crate::layout::LayoutCache;
use crate::loading::{FileId, Loader};

/// The core context which holds the loader, configuration and cached artifacts.
pub struct Context {
    /// The standard library scope.
    std: Scope,
    /// The style defaults.
    defaults: Defaults,
    /// The loader the context was created with.
    loader: Rc<dyn Loader>,
    /// Caches parsed font faces.
    pub fonts: FontCache,
    /// Caches decoded images.
    pub images: ImageCache,
    /// Caches evaluated modules.
    pub modules: ModuleCache,
    /// Caches layouting artifacts.
    #[cfg(feature = "layout-cache")]
    pub layouts: LayoutCache,
}

impl Context {
    /// Create a new context with the default settings.
    pub fn new(loader: Rc<dyn Loader>) -> Self {
        Self::builder().build(loader)
    }

    /// Create a new context with advanced settings.
    pub fn builder() -> ContextBuilder {
        ContextBuilder::default()
    }

    /// Garbage-collect caches.
    pub fn turnaround(&mut self) {
        #[cfg(feature = "layout-cache")]
        self.layouts.turnaround();
    }

    /// Typeset a source file into a collection of layouted frames.
    ///
    /// The `file` identifies the source file and is used to resolve relative
    /// paths (for importing and image loading).
    ///
    /// Returns a vector of frames representing individual pages alongside
    /// diagnostic information (errors and warnings).
    pub fn typeset(&mut self, file: FileId, src: &str) -> Pass<Vec<Rc<Frame>>> {
        let ast = parse::parse(src);
        let module = eval::eval(self, file, Rc::new(ast.output));
        let frames = layout::layout(self, &module.output.template.into_tree());

        let mut diags = ast.diags;
        diags.extend(module.diags);

        Pass::new(frames, diags)
    }
}

/// A builder for a [`Context`].
///
/// This struct is created by [`Context::builder`].
#[derive(Default)]
pub struct ContextBuilder {
    std: Option<Scope>,
    defaults: Option<Defaults>,
}

impl ContextBuilder {
    /// The scope containing definitions that are available everywhere
    /// (the standard library).
    pub fn std(mut self, std: Scope) -> Self {
        self.std = Some(std);
        self
    }

    /// The base properties for page size, font selection and so on.
    pub fn defaults(mut self, defaults: Defaults) -> Self {
        self.defaults = Some(defaults);
        self
    }

    /// Finish building the context by providing the `loader` used to load
    /// fonts, images, source files and other resources.
    pub fn build(self, loader: Rc<dyn Loader>) -> Context {
        Context {
            std: self.std.unwrap_or(library::new()),
            defaults: self.defaults.unwrap_or_default(),
            loader: Rc::clone(&loader),
            fonts: FontCache::new(Rc::clone(&loader)),
            images: ImageCache::new(loader),
            modules: ModuleCache::new(),
            #[cfg(feature = "layout-cache")]
            layouts: LayoutCache::new(),
        }
    }
}
