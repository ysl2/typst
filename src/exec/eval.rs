use super::table::SpannedEntry;
use super::{ExecCtx, TableValue, Value};
use crate::syntax::{Call, Deco, Expr, Spanned, TableExpr};

/// Evaluate an expression into an output value.
pub trait Eval {
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(self, env: &mut ExecCtx) -> Self::Output;
}

impl Eval for Call {
    type Output = Value;

    fn eval(self, ctx: &mut ExecCtx) -> Self::Output {
        let span = self.name.span;
        let name = self.name.v.as_str();

        if let Some(func) = ctx.funcs.get(name) {
            (*func.clone())(span, self.args, ctx)
        } else {
            if !name.is_empty() {
                error!(@ctx.f, span, "unknown function");
                ctx.f.decos.push(Spanned::new(Deco::Unresolved, span));
            }
            Value::Table(self.args.eval(ctx))
        }
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(self, ctx: &mut ExecCtx) -> Value {
        match self {
            Self::Ident(i) => Value::Ident(i),
            Self::Str(s) => Value::Str(s),
            Self::Bool(b) => Value::Bool(b),
            Self::Number(n) => Value::Number(n),
            Self::Length(s) => Value::Length(s),
            Self::Color(c) => Value::Color(c),
            Self::Table(t) => Value::Table(t.eval(ctx)),
            Self::Tree(t) => Value::Tree(ctx.process_tree(t)),
            Self::Call(call) => call.eval(ctx),
            Self::Neg(_) => todo!("eval neg"),
            Self::Add(_, _) => todo!("eval add"),
            Self::Sub(_, _) => todo!("eval sub"),
            Self::Mul(_, _) => todo!("eval mul"),
            Self::Div(_, _) => todo!("eval div"),
        }
    }
}

impl Eval for TableExpr {
    type Output = TableValue;

    fn eval(self, env: &mut ExecCtx) -> Self::Output {
        let mut table = TableValue::new();

        for (key, entry) in self.into_iter() {
            let val = entry.val.v.eval(env);
            let spanned = Spanned::new(val, entry.val.span);
            let entry = SpannedEntry::new(entry.key, spanned);
            table.insert(key, entry);
        }

        table
    }
}
