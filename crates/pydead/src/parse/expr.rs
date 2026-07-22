//! Expression tree walking for use collection.
use crate::symbols::NameUse;
use rustpython_parser::ast;

pub(crate) fn walk_arguments(
    args: &ast::Arguments,
    uses: &mut Vec<NameUse>,
    container: &Option<String>,
) {
    for arg in args
        .posonlyargs
        .iter()
        .chain(args.args.iter())
        .chain(args.kwonlyargs.iter())
    {
        if let Some(ann) = &arg.def.annotation {
            walk_expr(ann, uses, container);
        }
        if let Some(default) = &arg.default {
            walk_expr(default, uses, container);
        }
    }
    if let Some(vararg) = &args.vararg {
        if let Some(ann) = &vararg.annotation {
            walk_expr(ann, uses, container);
        }
    }
    if let Some(kwarg) = &args.kwarg {
        if let Some(ann) = &kwarg.annotation {
            walk_expr(ann, uses, container);
        }
    }
}

pub(crate) fn walk_store_target(
    expr: &ast::Expr,
    uses: &mut Vec<NameUse>,
    container: &Option<String>,
) {
    match expr {
        ast::Expr::Tuple(t) => {
            for e in &t.elts {
                walk_store_target(e, uses, container);
            }
        }
        ast::Expr::List(t) => {
            for e in &t.elts {
                walk_store_target(e, uses, container);
            }
        }
        ast::Expr::Starred(s) => walk_store_target(&s.value, uses, container),
        ast::Expr::Subscript(s) => {
            walk_expr(&s.value, uses, container);
            walk_expr(&s.slice, uses, container);
        }
        ast::Expr::Attribute(a) => walk_expr(&a.value, uses, container),
        _ => {}
    }
}

pub(crate) fn walk_expr(expr: &ast::Expr, uses: &mut Vec<NameUse>, container: &Option<String>) {
    match expr {
        ast::Expr::Name(n) => {
            if matches!(n.ctx, ast::ExprContext::Load) {
                uses.push(NameUse {
                    name: n.id.to_string(),
                    module: None,
                    imported: None,
                    container: container.clone(),
                });
            }
        }
        ast::Expr::Attribute(a) => {
            walk_expr(&a.value, uses, container);
            uses.push(NameUse {
                name: a.attr.to_string(),
                module: None,
                imported: None,
                container: container.clone(),
            });
        }
        ast::Expr::Call(c) => {
            walk_expr(&c.func, uses, container);
            for a in &c.args {
                walk_expr(a, uses, container);
            }
            for kw in &c.keywords {
                walk_expr(&kw.value, uses, container);
            }
        }
        ast::Expr::BinOp(b) => {
            walk_expr(&b.left, uses, container);
            walk_expr(&b.right, uses, container);
        }
        ast::Expr::UnaryOp(u) => walk_expr(&u.operand, uses, container),
        ast::Expr::BoolOp(b) => {
            for v in &b.values {
                walk_expr(v, uses, container);
            }
        }
        ast::Expr::Compare(c) => {
            walk_expr(&c.left, uses, container);
            for c in &c.comparators {
                walk_expr(c, uses, container);
            }
        }
        ast::Expr::IfExp(i) => {
            walk_expr(&i.test, uses, container);
            walk_expr(&i.body, uses, container);
            walk_expr(&i.orelse, uses, container);
        }
        ast::Expr::Dict(d) => {
            for k in d.keys.iter().flatten() {
                walk_expr(k, uses, container);
            }
            for v in &d.values {
                walk_expr(v, uses, container);
            }
        }
        ast::Expr::Set(s) => {
            for e in &s.elts {
                walk_expr(e, uses, container);
            }
        }
        ast::Expr::ListComp(l) => {
            walk_expr(&l.elt, uses, container);
            for g in &l.generators {
                walk_expr(&g.iter, uses, container);
                for i in &g.ifs {
                    walk_expr(i, uses, container);
                }
            }
        }
        ast::Expr::SetComp(l) => {
            walk_expr(&l.elt, uses, container);
            for g in &l.generators {
                walk_expr(&g.iter, uses, container);
                for i in &g.ifs {
                    walk_expr(i, uses, container);
                }
            }
        }
        ast::Expr::GeneratorExp(l) => {
            walk_expr(&l.elt, uses, container);
            for g in &l.generators {
                walk_expr(&g.iter, uses, container);
                for i in &g.ifs {
                    walk_expr(i, uses, container);
                }
            }
        }
        ast::Expr::DictComp(l) => {
            walk_expr(&l.key, uses, container);
            walk_expr(&l.value, uses, container);
            for g in &l.generators {
                walk_expr(&g.iter, uses, container);
                for i in &g.ifs {
                    walk_expr(i, uses, container);
                }
            }
        }
        ast::Expr::Await(a) => walk_expr(&a.value, uses, container),
        ast::Expr::Yield(y) => {
            if let Some(v) = &y.value {
                walk_expr(v, uses, container);
            }
        }
        ast::Expr::YieldFrom(y) => walk_expr(&y.value, uses, container),
        ast::Expr::JoinedStr(j) => {
            for v in &j.values {
                walk_expr(v, uses, container);
            }
        }
        ast::Expr::FormattedValue(f) => {
            walk_expr(&f.value, uses, container);
            if let Some(fmt) = &f.format_spec {
                walk_expr(fmt, uses, container);
            }
        }
        ast::Expr::Constant(_) => {}
        ast::Expr::Subscript(s) => {
            walk_expr(&s.value, uses, container);
            walk_expr(&s.slice, uses, container);
        }
        ast::Expr::Starred(s) => walk_expr(&s.value, uses, container),
        ast::Expr::List(l) => {
            for e in &l.elts {
                walk_expr(e, uses, container);
            }
        }
        ast::Expr::Tuple(t) => {
            for e in &t.elts {
                walk_expr(e, uses, container);
            }
        }
        ast::Expr::Slice(s) => {
            if let Some(l) = &s.lower {
                walk_expr(l, uses, container);
            }
            if let Some(u) = &s.upper {
                walk_expr(u, uses, container);
            }
            if let Some(st) = &s.step {
                walk_expr(st, uses, container);
            }
        }
        ast::Expr::Lambda(l) => {
            walk_arguments(&l.args, uses, container);
            walk_expr(&l.body, uses, container);
        }
        ast::Expr::NamedExpr(n) => {
            walk_expr(&n.value, uses, container);
        }
    }
}
