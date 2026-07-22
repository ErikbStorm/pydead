# Azure Functions Python v2 fixture

End-to-end ground truth for PyDead’s Azure Functions decorator handling.

## Entry points (must stay live)

Decorated with `@app.*` / `@bp.*` / `@df_app.*` triggers — never called from Python:

| Function | Decorator |
|----------|-----------|
| `http_hello` | `@app.route` (+ `function_name`) |
| `timer_cleanup` | `@app.timer_trigger` |
| `queue_worker` | `@app.queue_trigger` |
| `blob_processor` | `@app.blob_trigger` |
| `service_bus_worker` | `@app.service_bus_queue_trigger` |
| `event_hub_worker` | `@app.event_hub_trigger` |
| `cosmos_worker` | `@app.cosmos_db_trigger` |
| `event_grid_worker` | `@app.event_grid_trigger` |
| `hello_orchestrator` | `@df_app.orchestration_trigger` |
| `say_hello` | `@df_app.activity_trigger` |
| `get_order` | `@bp.route` (Blueprint) |

Helpers only used from those entry points (`format_message`, `used_by_http`) stay live via iterative analysis.

## Dead (must be reported)

- `completely_unused_helper`, `blueprint_dead_helper`, `unused_orphan`, `also_unused`

See `EXPECTED.json`.
