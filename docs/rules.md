# PyDead rule catalog

PyDead uses short **rule codes** (like Ruff) in two families:

| Prefix | Meaning |
|--------|---------|
| **EP** | **Entry-point exemption** — keep a definition *live* even if nothing in the folder calls it (framework/CLI/migration hooks). |
| **DC** | **Dead-code finding** — report an unused definition. |

Findings look like:

```text
alembic/versions/001.py:14:1: DC001 Function 'helper' is never referenced in the workspace (confidence 70)
```

JSON includes `"code": "DC001"`.

---

## DC — dead code findings

| Code | Name | What it reports |
|------|------|-----------------|
| **DC001** | `unused-function` | Unused function |
| **DC002** | `unused-class` | Unused class |
| **DC003** | `unused-method` | Unused method |
| **DC004** | `unused-variable` | Unused module-level variable |

Disable a finding type:

```toml
[tool.pydead]
ignore = ["DC004"]  # do not report unused module-level variables
```

---

## EP — entry-point exemptions

These rules mark definitions as **live roots**. Anything they call stays live via iterative analysis.

| Code | Name | Default | What it keeps live |
|------|------|---------|-------------------|
| **EP001** | `dunder-names` | on | `__init__`, `__str__`, other `__*__` |
| **EP002** | `test-discovery` | on | `test_*`, `Test*` classes, `pytest_*` |
| **EP003** | `dunder-all-exports` | on | Names listed in module `__all__` |
| **EP004** | `azure-functions-v2` | on | `@app.route`, `@app.timer_trigger`, `@app.activity_trigger`, `@bp.*`, any `*_trigger`, … |
| **EP005** | `alembic-migrations` | on | `upgrade` / `downgrade` plus metadata `revision`, `down_revision`, `branch_labels`, `depends_on` under Alembic-style paths |
| **EP006** | `pydantic-hooks` | on | Pydantic v1/v2 validators, serializers, computed fields, schema hooks (`model_post_init`, `__get_pydantic_*__`, …) |
| **EP007** | `sqlalchemy-hooks` | on | SQLAlchemy TypeDecorator/TypeEngine methods, ORM `@validates` / hybrids / `@declared_attr`, `@compiles`, events |
| **EP010** | `user-entry-names` | on | Config `entry_names` |
| **EP011** | `user-entry-decorators` | on | Config `entry_decorators` |
| **EP012** | `user-entry-rules` | on | Config `[[tool.pydead.entry_rules]]` |

Disable a built-in exemption:

```toml
[tool.pydead]
ignore = ["EP005"]  # treat Alembic upgrade/downgrade as normal functions
```

### EP005 — Alembic (default paths)

Symbols named `upgrade`, `downgrade`, `revision`, `down_revision`, `branch_labels`, or `depends_on` whose file path matches any of:

- `**/versions/*.py`
- `**/versions/**/*.py`
- `**/alembic/**/*.py`
- `**/migrations/versions/*.py`
- `**/migrations/**/*.py`

Override paths:

```toml
[tool.pydead]
alembic_paths = ["**/db/revisions/*.py"]
```

### EP006 — Pydantic

Pydantic invokes these without explicit call sites in your code.

**Decorators** (attribute or bare import name):

| Decorator | Notes |
|-----------|--------|
| `field_validator` | v2 field validators (`mode='before'\|'after'\|'wrap'\|'plain'`) |
| `model_validator` | v2 model-level validators |
| `field_serializer` / `model_serializer` | v2 serializers |
| `computed_field` | v2 computed fields |
| `validate_call` | v2 validated callables |
| `validator` / `root_validator` | v1 |
| `validate_arguments` | v1 |
| any `*_validator` / `*_serializer` | forward-compatible suffix match |

**Special names** (called by convention):

| Name | Notes |
|------|--------|
| `model_post_init` | v2 post-init hook |
| `__get_pydantic_core_schema__` | custom types (v2) |
| `__get_pydantic_json_schema__` | JSON schema customization (v2) |
| `__get_validators__` / `__modify_schema__` | v1 custom types |
| `__pydantic_init_subclass__` | subclass hook |

**Already covered without EP006:** functions passed to `AfterValidator(fn)`, `BeforeValidator(fn)`, etc. are normal name references and stay live via analysis.

Disable:

```toml
[tool.pydead]
ignore = ["EP006"]
```

Fixture: `fixtures/pydantic_project/`.

### EP007 — SQLAlchemy

Hooks SQLAlchemy calls when a type/mapper is **used**, without explicit Python call sites.

**TypeDecorator / TypeEngine method names** (non-exhaustive but covers common cases):

`load_dialect_impl`, `bind_expression`, `column_expression`, `process_bind_param`, `process_result_value`, `process_literal_param`, `bind_processor`, `result_processor`, `literal_processor`, `compare_values`, `coerce_compared_value`, `get_col_spec`, `get_dbapi_type`, `as_generic`, `copy`, `dialect_impl`, `python_type`, `comparator_factory`, …

**ORM / registration decorators:** `@validates`, `@reconstructor`, `@declared_attr`, `@hybrid_property`, `@hybrid_method`, `@compiles`, `@event.listens_for` / `listens_for`.

**Names:** `__mapper_args__`, `__table_args__`, `__tablename__`, `__abstract__`.

Example (`BytesGeometry`): `load_dialect_impl`, `column_expression`, and `bind_expression` stay live via EP007 when the class is referenced as a column type.

Fixture: `fixtures/sqlalchemy_project/`.

---

## Mark a definition as “not unused” (inline keep)

### VS Code Quick Fix

On a PyDead diagnostic, open the lightbulb (`Cmd+.` / `Ctrl+.`) and choose:

- **PyDead: keep '…' (mark as used)** — inserts `# pydead: keep` (preferred)
- **PyDead: keep (DCxxx only)** — inserts `# pydead: keep DCxxx`
- Or command palette: **PyDead: Keep (mark as used)**

### Manual comments

If PyDead reports something you intentionally keep (plugin hooks, string-dispatched
callables, etc.), mark it on the **def line**, the **line above**, or a **decorator** above it:

```python
# Recommended — explicit “this is used outside static analysis”
def leftover_for_plugins() -> None:  # pydead: keep
    ...

def also_ok() -> None:  # pydead: used
    ...

# Ruff-compatible
def ruff_style() -> None:  # noqa: DC001
    ...

# Only suppress unused-method
def weird_hook(self) -> None:  # pydead: keep DC003
    ...

# Above the def (works through decorator stacks)
# pydead: keep
@app.task
def celery_style():
    ...
```

| Form | Effect |
|------|--------|
| `# pydead: keep` | **Preferred** — keep definition (all DC codes) |
| `# pydead: used` | Same as keep |
| `# pydead: allow` | Same as keep |
| `# pydead: ignore` / `# noqa` | Same — suppress all DC on this def |
| `# pydead: keep DC003` / `# noqa: DC003` | Only that finding code |
| `# pydead: ignore[DC001,DC003]` | Multiple codes |

Scans the definition line, decorator lines above it, blanks, and comment-only lines.

This only hides **findings** (DC*). It does not change EP entry-point rules.

---

## Adding your own exemptions

### 1. By function name (EP010)

```toml
[tool.pydead]
entry_names = [
  "main",
  "cli",
  "handle_*",      # glob
  "on_event",
]
```

### 2. By decorator attribute (EP011)

Matches the **last** attribute of the decorator, e.g. `@app.task` → `task`, `@shared_task` → use name form if bare.

```toml
[tool.pydead]
entry_decorators = [
  "task",           # Celery @app.task / @shared_task (attr)
  "shared_task",
  "receiver",       # Django signals
  "command",        # Click sometimes uses @click.command — attr is "command"
  "*_view",
]
```

### 3. Path-scoped custom rules (EP012)

Use a **custom code** so teammates can ignore or document it:

```toml
[[tool.pydead.entry_rules]]
code = "EP100"
description = "Prefect flow entrypoints"
names = ["my_flow", "etl_*"]
paths = ["**/flows/**/*.py"]

[[tool.pydead.entry_rules]]
code = "EP101"
description = "Click commands"
decorators = ["command", "group"]
paths = ["**/cli/**/*.py"]
```

- `code` — required for custom rules (shown in docs / future explain); must be unique.
- `names` — exact or `*` globs.
- `decorators` — decorator attribute names / globs.
- `paths` — optional; if empty, applies everywhere. Supports `**` and `*`.

### 4. Ignore specific definition names entirely

Not an entry point — suppresses the finding if it would still be dead:

```toml
[tool.pydead]
ignore_names = ["visit_*", "do_*"]
```

---

## Full config example

```toml
[tool.pydead]
min_confidence = 70
keep_public = false

# Turn off rules you do not want
ignore = []  # e.g. ["DC004", "EP002"]

# Extra entry points
entry_names = ["main"]
entry_decorators = ["task"]

# Alembic path override (optional)
# alembic_paths = ["**/versions/*.py"]

[[tool.pydead.entry_rules]]
code = "EP100"
description = "My framework handlers"
names = ["handle"]
paths = ["**/handlers/*.py"]
```

Also supported in `pyproject.toml` under `[tool.pydead]` with the same keys / `[[tool.pydead.entry_rules]]`.

---

## CLI

```bash
# List all built-in rule codes
pydead rules

# Analyze (codes appear in text + JSON)
pydead find .
pydead find . --format json
```

---

## Design notes

- **EP** rules only affect *liveness*. They do not emit diagnostics.
- **DC** rules are the diagnostics. Ignore a DC code to silence that kind of finding.
- Prefer a named **EP** rule (or custom `EP1xx`) over broad `ignore_names` so intent is clear in config reviews.
