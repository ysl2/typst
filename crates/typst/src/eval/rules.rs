use crate::diag::{At, SourceResult};
use crate::eval::{Eval, Vm};
use crate::foundations::{
    Func, Recipe, Revocation, ShowableSelector, Style, Styles, Transformation,
};
use crate::syntax::ast::{self, AstNode};

impl Eval for ast::SetRule<'_> {
    type Output = Styles;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if let Some(condition) = self.condition() {
            if !condition.eval(vm)?.cast::<bool>().at(condition.span())? {
                return Ok(Styles::new());
            }
        }

        let target = self.target();
        let target = target
            .eval(vm)?
            .cast::<Func>()
            .and_then(|func| {
                func.element().ok_or_else(|| {
                    "only element functions can be used in set rules".into()
                })
            })
            .at(target.span())?;
        let args = self.args().eval(vm)?;
        Ok(target.set(&mut vm.engine, args)?.spanned(self.span()))
    }
}

impl Eval for ast::ShowRule<'_> {
    type Output = Recipe;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let selector = self
            .selector()
            .map(|sel| sel.eval(vm)?.cast::<ShowableSelector>().at(sel.span()))
            .transpose()?
            .map(|selector| selector.0);

        let transform = self.transform();
        let span = transform.span();

        let transform = match transform {
            ast::Expr::Set(set) => Transformation::Style(set.eval(vm)?),
            ast::Expr::Revoke(revoke) => {
                Transformation::Style(Style::from(revoke.eval(vm)?).into())
            }
            expr => expr.eval(vm)?.cast::<Transformation>().at(span)?,
        };

        Ok(Recipe { span, selector, transform })
    }
}

impl Eval for ast::RevokeRule<'_> {
    type Output = Revocation;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let selector = self.selector();
        Ok(Revocation {
            span: self.span(),
            selector: selector
                .eval(vm)?
                .cast::<ShowableSelector>()
                .at(selector.span())?
                .0,
        })
    }
}
