mod expr;

use std::path::{Path, PathBuf};

use rustpython_parser::ast::{self, Suite};
use rustpython_parser::source_code::{LineIndex, SourceCode};
use rustpython_parser::text_size::TextRange;
use rustpython_parser::Parse;

use crate::discover::path_to_module;
use crate::symbols::{DefKind, Definition, NameUse, Position, Range};
use self::expr::{walk_arguments, walk_expr, walk_store_target};

/// A successfully parsed Python module.
pub struct ParsedModule {
    pub path: PathBuf,
    pub rel_path: String,
    pub module: String,
    pub source: String,
    pub suite: Suite,
    pub line_index: LineIndex,
}

pub struct Extracted {
    pub definitions: Vec<Definition>,
    pub uses: Vec<NameUse>,
    /// Names forced live via `__all__`.
    pub all_exports: Vec<String>,
}

pub fn parse_file(root: &Path, path: &Path) -> anyhow::Result<ParsedModule> {
    if !crate::path_safety::file_allowed_for_read(path).unwrap_or(false) {
        anyhow::bail!(
            "skipping unsafe or oversized file: {}",
            path.display()
        );
    }
    let source = std::fs::read_to_string(path)?;
    let path_str = path.to_string_lossy();
    let suite = Suite::parse(&source, &path_str)
        .map_err(|e| anyhow::anyhow!("parse error in {}: {}", path.display(), e))?;
    let line_index = LineIndex::from_source_text(&source);
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let module = path_to_module(root, path);
    Ok(ParsedModule {
        path: path.to_path_buf(),
        rel_path: rel,
        module,
        source,
        suite,
        line_index,
    })
}

pub fn extract(module: &ParsedModule) -> Extracted {
    let mut definitions = Vec::new();
    let mut uses = Vec::new();
    let mut all_exports = Vec::new();
    let code = SourceCode::new(&module.source, &module.line_index);
    let container: Option<String> = None;

    walk_body(
        &module.suite,
        module,
        &code,
        &mut definitions,
        &mut uses,
        &mut all_exports,
        None,
        true,
        &container,
    );

    Extracted {
        definitions,
        uses,
        all_exports,
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_body(
    body: &[ast::Stmt],
    module: &ParsedModule,
    code: &SourceCode<'_, '_>,
    definitions: &mut Vec<Definition>,
    uses: &mut Vec<NameUse>,
    all_exports: &mut Vec<String>,
    class_stack: Option<&str>,
    is_module_level: bool,
    container: &Option<String>,
) {
    for stmt in body {
        walk_stmt(
            stmt,
            module,
            code,
            definitions,
            uses,
            all_exports,
            class_stack,
            is_module_level,
            container,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_stmt(
    stmt: &ast::Stmt,
    module: &ParsedModule,
    code: &SourceCode<'_, '_>,
    definitions: &mut Vec<Definition>,
    uses: &mut Vec<NameUse>,
    all_exports: &mut Vec<String>,
    class_stack: Option<&str>,
    is_module_level: bool,
    container: &Option<String>,
) {
    match stmt {
        ast::Stmt::FunctionDef(f) => {
            let kind = if class_stack.is_some() {
                DefKind::Method
            } else {
                DefKind::Function
            };
            let name = f.name.to_string();
            let qualname = match class_stack {
                Some(cls) => format!("{}.{}.{}", module.module, cls, name),
                None => format!("{}.{}", module.module, name),
            };
            record_function(
                definitions,
                uses,
                all_exports,
                module,
                code,
                class_stack,
                container,
                &name,
                &qualname,
                kind,
                f.range,
                &f.decorator_list,
                &f.args,
                f.returns.as_deref(),
                &f.body,
            );
        }
        ast::Stmt::AsyncFunctionDef(f) => {
            let kind = if class_stack.is_some() {
                DefKind::Method
            } else {
                DefKind::Function
            };
            let name = f.name.to_string();
            let qualname = match class_stack {
                Some(cls) => format!("{}.{}.{}", module.module, cls, name),
                None => format!("{}.{}", module.module, name),
            };
            record_function(
                definitions,
                uses,
                all_exports,
                module,
                code,
                class_stack,
                container,
                &name,
                &qualname,
                kind,
                f.range,
                &f.decorator_list,
                &f.args,
                f.returns.as_deref(),
                &f.body,
            );
        }
        ast::Stmt::ClassDef(c) => {
            let name = c.name.to_string();
            let qualname = match class_stack {
                Some(cls) => format!("{}.{}.{}", module.module, cls, name),
                None => format!("{}.{}", module.module, name),
            };
            push_def(
                definitions,
                module,
                code,
                DefKind::Class,
                &name,
                &qualname,
                c.range,
                collect_decorator_attrs(&c.decorator_list),
            );
            for d in &c.decorator_list {
                walk_expr(d, uses, container);
            }
            for b in &c.bases {
                walk_expr(b, uses, container);
            }
            for kw in &c.keywords {
                walk_expr(&kw.value, uses, container);
            }
            let class_name = name.clone();
            let inner = Some(qualname);
            walk_body(
                &c.body,
                module,
                code,
                definitions,
                uses,
                all_exports,
                Some(&class_name),
                false,
                &inner,
            );
        }
        ast::Stmt::Assign(a) if is_module_level => {
            let is_all = a
                .targets
                .iter()
                .any(|t| matches!(t, ast::Expr::Name(n) if n.id.as_str() == "__all__"));
            if is_all {
                collect_all_exports(&a.value, all_exports);
            }
            for target in &a.targets {
                collect_module_assign_targets(target, module, code, definitions, a.range);
            }
            walk_expr(&a.value, uses, container);
        }
        ast::Stmt::AnnAssign(a) if is_module_level => {
            if let ast::Expr::Name(n) = a.target.as_ref() {
                if n.id.as_str() == "__all__" {
                    if let Some(v) = &a.value {
                        collect_all_exports(v, all_exports);
                    }
                } else {
                    let name = n.id.to_string();
                    let qualname = format!("{}.{}", module.module, name);
                    push_def(
                        definitions,
                        module,
                        code,
                        DefKind::Variable,
                        &name,
                        &qualname,
                        a.range,
                        vec![],
                    );
                }
            }
            walk_expr(&a.annotation, uses, container);
            if let Some(v) = &a.value {
                walk_expr(v, uses, container);
            }
        }
        ast::Stmt::Assign(a) => {
            walk_expr(&a.value, uses, container);
            for t in &a.targets {
                walk_store_target(t, uses, container);
            }
        }
        ast::Stmt::AnnAssign(a) => {
            walk_expr(&a.annotation, uses, container);
            if let Some(v) = &a.value {
                walk_expr(v, uses, container);
            }
        }
        ast::Stmt::AugAssign(a) => {
            walk_expr(&a.value, uses, container);
            walk_expr(&a.target, uses, container);
        }
        ast::Stmt::Import(i) => {
            for alias in &i.names {
                let mod_name = alias.name.to_string();
                uses.push(NameUse {
                    name: mod_name.clone(),
                    module: Some(mod_name.clone()),
                    imported: None,
                    container: container.clone(),
                });
                if let Some(last) = mod_name.rsplit('.').next() {
                    uses.push(NameUse {
                        name: last.to_string(),
                        module: Some(mod_name),
                        imported: None,
                        container: container.clone(),
                    });
                }
            }
        }
        ast::Stmt::ImportFrom(i) => {
            let base = resolve_import_from_module(module, i);
            for alias in &i.names {
                let imported_name = alias.name.to_string();
                if imported_name == "*" {
                    if let Some(ref base) = base {
                        uses.push(NameUse {
                            name: base.clone(),
                            module: Some(base.clone()),
                            imported: None,
                            container: container.clone(),
                        });
                    }
                    continue;
                }
                if let Some(ref base) = base {
                    uses.push(NameUse {
                        name: imported_name.clone(),
                        module: Some(base.clone()),
                        imported: Some((base.clone(), imported_name)),
                        container: container.clone(),
                    });
                } else {
                    uses.push(NameUse {
                        name: imported_name,
                        module: None,
                        imported: None,
                        container: container.clone(),
                    });
                }
            }
        }
        ast::Stmt::For(s) => {
            walk_expr(&s.iter, uses, container);
            walk_body(
                &s.body,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                false,
                container,
            );
            walk_body(
                &s.orelse,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                false,
                container,
            );
        }
        ast::Stmt::AsyncFor(s) => {
            walk_expr(&s.iter, uses, container);
            walk_body(
                &s.body,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                false,
                container,
            );
            walk_body(
                &s.orelse,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                false,
                container,
            );
        }
        ast::Stmt::While(s) => {
            walk_expr(&s.test, uses, container);
            walk_body(
                &s.body,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                false,
                container,
            );
            walk_body(
                &s.orelse,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                false,
                container,
            );
        }
        ast::Stmt::If(s) => {
            walk_expr(&s.test, uses, container);
            walk_body(
                &s.body,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                is_module_level,
                container,
            );
            walk_body(
                &s.orelse,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                is_module_level,
                container,
            );
        }
        ast::Stmt::With(s) => {
            for item in &s.items {
                walk_expr(&item.context_expr, uses, container);
            }
            walk_body(
                &s.body,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                false,
                container,
            );
        }
        ast::Stmt::AsyncWith(s) => {
            for item in &s.items {
                walk_expr(&item.context_expr, uses, container);
            }
            walk_body(
                &s.body,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                false,
                container,
            );
        }
        ast::Stmt::Match(s) => {
            walk_expr(&s.subject, uses, container);
            for case in &s.cases {
                if let Some(guard) = &case.guard {
                    walk_expr(guard, uses, container);
                }
                walk_body(
                    &case.body,
                    module,
                    code,
                    definitions,
                    uses,
                    all_exports,
                    class_stack,
                    false,
                    container,
                );
            }
        }
        ast::Stmt::Try(_) | ast::Stmt::TryStar(_) => {
            walk_try(
                stmt,
                module,
                code,
                definitions,
                uses,
                all_exports,
                class_stack,
                is_module_level,
                container,
            );
        }
        ast::Stmt::Raise(s) => {
            if let Some(e) = &s.exc {
                walk_expr(e, uses, container);
            }
            if let Some(c) = &s.cause {
                walk_expr(c, uses, container);
            }
        }
        ast::Stmt::Assert(s) => {
            walk_expr(&s.test, uses, container);
            if let Some(m) = &s.msg {
                walk_expr(m, uses, container);
            }
        }
        ast::Stmt::Expr(s) => walk_expr(&s.value, uses, container),
        ast::Stmt::Return(s) => {
            if let Some(v) = &s.value {
                walk_expr(v, uses, container);
            }
        }
        ast::Stmt::Delete(s) => {
            for t in &s.targets {
                walk_expr(t, uses, container);
            }
        }
        ast::Stmt::Global(_)
        | ast::Stmt::Nonlocal(_)
        | ast::Stmt::Pass(_)
        | ast::Stmt::Break(_)
        | ast::Stmt::Continue(_) => {}
        ast::Stmt::TypeAlias(s) => {
            walk_expr(&s.name, uses, container);
            walk_expr(&s.value, uses, container);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_try(
    stmt: &ast::Stmt,
    module: &ParsedModule,
    code: &SourceCode<'_, '_>,
    definitions: &mut Vec<Definition>,
    uses: &mut Vec<NameUse>,
    all_exports: &mut Vec<String>,
    class_stack: Option<&str>,
    is_module_level: bool,
    container: &Option<String>,
) {
    let (body, handlers, orelse, finalbody) = match stmt {
        ast::Stmt::Try(s) => (&s.body, &s.handlers, &s.orelse, &s.finalbody),
        ast::Stmt::TryStar(s) => (&s.body, &s.handlers, &s.orelse, &s.finalbody),
        _ => return,
    };
    walk_body(
        body,
        module,
        code,
        definitions,
        uses,
        all_exports,
        class_stack,
        is_module_level,
        container,
    );
    for h in handlers {
        let ast::ExceptHandler::ExceptHandler(h) = h;
        if let Some(t) = &h.type_ {
            walk_expr(t, uses, container);
        }
        walk_body(
            &h.body,
            module,
            code,
            definitions,
            uses,
            all_exports,
            class_stack,
            false,
            container,
        );
    }
    walk_body(
        orelse,
        module,
        code,
        definitions,
        uses,
        all_exports,
        class_stack,
        is_module_level,
        container,
    );
    walk_body(
        finalbody,
        module,
        code,
        definitions,
        uses,
        all_exports,
        class_stack,
        is_module_level,
        container,
    );
}

fn collect_module_assign_targets(
    target: &ast::Expr,
    module: &ParsedModule,
    code: &SourceCode<'_, '_>,
    definitions: &mut Vec<Definition>,
    range: TextRange,
) {
    match target {
        ast::Expr::Name(n) => {
            let name = n.id.to_string();
            if name == "__all__" || is_dunder(&name) {
                return;
            }
            let qualname = format!("{}.{}", module.module, name);
            push_def(
                definitions,
                module,
                code,
                DefKind::Variable,
                &name,
                &qualname,
                range,
                vec![],
            );
        }
        ast::Expr::Tuple(t) => {
            for e in &t.elts {
                collect_module_assign_targets(e, module, code, definitions, range);
            }
        }
        ast::Expr::List(t) => {
            for e in &t.elts {
                collect_module_assign_targets(e, module, code, definitions, range);
            }
        }
        _ => {}
    }
}

fn collect_all_exports(value: &ast::Expr, all_exports: &mut Vec<String>) {
    let elts = match value {
        ast::Expr::List(l) => &l.elts,
        ast::Expr::Tuple(t) => &t.elts,
        _ => return,
    };
    for e in elts {
        if let Some(s) = const_str(e) {
            all_exports.push(s);
        }
    }
}

fn const_str(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Constant(c) => match &c.value {
            ast::Constant::Str(s) => Some(s.clone()),
            _ => None,
        },
        _ => None,
    }
}


fn resolve_import_from_module(module: &ParsedModule, imp: &ast::StmtImportFrom) -> Option<String> {
    let level = imp.level.map(|l| l.to_u32()).unwrap_or(0);
    let module_name = imp.module.as_ref().map(|m| m.to_string());

    if level == 0 {
        return module_name;
    }

    let mut owned: Vec<String> = module.module.split('.').map(|s| s.to_string()).collect();
    let is_init = module.path.file_name().and_then(|f| f.to_str()) == Some("__init__.py");
    if !is_init && !owned.is_empty() {
        owned.pop();
    }
    let up = (level as usize).saturating_sub(1);
    for _ in 0..up {
        owned.pop();
    }
    if let Some(m) = module_name {
        if !m.is_empty() {
            for part in m.split('.') {
                owned.push(part.to_string());
            }
        }
    }
    if owned.is_empty() {
        None
    } else {
        Some(owned.join("."))
    }
}

#[allow(clippy::too_many_arguments)]
fn record_function(
    definitions: &mut Vec<Definition>,
    uses: &mut Vec<NameUse>,
    all_exports: &mut Vec<String>,
    module: &ParsedModule,
    code: &SourceCode<'_, '_>,
    class_stack: Option<&str>,
    container: &Option<String>,
    name: &str,
    qualname: &str,
    kind: DefKind,
    range: TextRange,
    decorator_list: &[ast::Expr],
    args: &ast::Arguments,
    returns: Option<&ast::Expr>,
    body: &[ast::Stmt],
) {
    let decorator_attrs = collect_decorator_attrs(decorator_list);
    push_def(
        definitions,
        module,
        code,
        kind,
        name,
        qualname,
        range,
        decorator_attrs,
    );
    for d in decorator_list {
        walk_expr(d, uses, container);
    }
    let inner = Some(qualname.to_string());
    walk_arguments(args, uses, &inner);
    if let Some(ret) = returns {
        walk_expr(ret, uses, &inner);
    }
    walk_body(
        body,
        module,
        code,
        definitions,
        uses,
        all_exports,
        class_stack,
        false,
        &inner,
    );
}

fn collect_decorator_attrs(decorators: &[ast::Expr]) -> Vec<String> {
    let mut attrs = Vec::new();
    for d in decorators {
        if let Some(name) = decorator_attr_name(d) {
            attrs.push(name);
        }
    }
    attrs
}

fn decorator_attr_name(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Attribute(a) => Some(a.attr.to_string()),
        ast::Expr::Call(c) => decorator_attr_name(&c.func),
        ast::Expr::Name(n) => Some(n.id.to_string()),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_def(
    definitions: &mut Vec<Definition>,
    module: &ParsedModule,
    code: &SourceCode<'_, '_>,
    kind: DefKind,
    name: &str,
    qualname: &str,
    range: TextRange,
    decorator_attrs: Vec<String>,
) {
    let start_loc = code.source_location(range.start());
    let end_loc = code.source_location(range.end());
    let lsp_range = Range {
        start: Position {
            line: start_loc.row.get().saturating_sub(1),
            character: start_loc.column.get().saturating_sub(1),
        },
        end: Position {
            line: end_loc.row.get().saturating_sub(1),
            character: end_loc.column.get().saturating_sub(1),
        },
    };
    definitions.push(Definition {
        kind,
        name: name.to_string(),
        qualname: qualname.to_string(),
        module: module.module.clone(),
        path: module.path.clone(),
        rel_path: module.rel_path.clone(),
        byte_start: u32::from(range.start()),
        byte_end: u32::from(range.end()),
        range: lsp_range,
        is_private: name.starts_with('_') && !is_dunder(name),
        decorator_attrs,
    });
}

pub fn is_dunder(name: &str) -> bool {
    name.starts_with("__") && name.ends_with("__") && name.len() >= 4
}

/// Expand a statement range to include a trailing newline (and one blank line)
/// for cleaner deletion.
pub fn expand_removal_bytes(source: &str, start: u32, end: u32) -> (u32, u32) {
    let bytes = source.as_bytes();
    let mut end = end as usize;
    if end < bytes.len() && (bytes[end] == b'\n' || bytes[end] == b'\r') {
        if bytes[end] == b'\r' && end + 1 < bytes.len() && bytes[end + 1] == b'\n' {
            end += 2;
        } else {
            end += 1;
        }
    }
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    } else if end + 1 < bytes.len() && bytes[end] == b'\r' && bytes[end + 1] == b'\n' {
        end += 2;
    }
    (start, end as u32)
}
