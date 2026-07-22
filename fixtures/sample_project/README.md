# Sample monorepo for pydead integration tests

Hand-authored multi-package Python project with **intentional** live and dead code.

## Layout

| Path | Intent |
|------|--------|
| `libs/core/api.py` | Live `Greeter` + dead orphans / `DeadService` / unused method |
| `libs/core/util.py` | Live `USED_CONSTANT` / `format_name`; dead `UNUSED_CONSTANT` / `dead_helper` |
| `libs/core/__init__.py` | `__all__` keeps `reexported_via_all` live |
| `libs/plugins/legacy.py` | Fully unused plugin surface |
| `apps/greeter/main.py` | Cross-package consumer of live APIs only |
| `apps/greeter/unused_app.py` | Module-local call graph; `also_never_called` is dead from outside |

Ground truth: [`EXPECTED.json`](./EXPECTED.json).
