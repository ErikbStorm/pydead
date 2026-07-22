"""
Azure Functions Python programming model v2 — entry module.

All trigger handlers are discovered by the host via decorators; nothing in
this repo calls them directly. PyDead must treat them as live entry points.
"""

import azure.functions as func
import azure.durable_functions as df
import logging

from blueprints.orders import bp
from shared.helpers import format_message, used_by_http

app = func.FunctionApp(http_auth_level=func.AuthLevel.ANONYMOUS)
app.register_functions(bp)

# Durable Functions app (same process / extension bundle in real deployments)
df_app = df.DFApp(http_auth_level=func.AuthLevel.ANONYMOUS)


@app.function_name(name="HttpHello")
@app.route(route="hello", methods=["GET", "POST"])
def http_hello(req: func.HttpRequest) -> func.HttpResponse:
    """HTTP trigger — host entry point."""
    name = req.params.get("name") or "world"
    return func.HttpResponse(format_message(name), status_code=200)


@app.timer_trigger(
    schedule="0 */5 * * * *",
    arg_name="mytimer",
    run_on_startup=False,
    use_monitor=False,
)
def timer_cleanup(mytimer: func.TimerRequest) -> None:
    """Timer trigger — host entry point."""
    logging.info("timer fired: past_due=%s", mytimer.past_due)
    used_by_http("timer")


@app.queue_trigger(
    arg_name="msg",
    queue_name="jobs",
    connection="AzureWebJobsStorage",
)
def queue_worker(msg: func.QueueMessage) -> None:
    """Queue trigger — host entry point."""
    logging.info("queue message: %s", msg.get_body().decode("utf-8"))


@app.blob_trigger(
    arg_name="blob",
    path="samples-workitems/{name}",
    connection="AzureWebJobsStorage",
)
def blob_processor(blob: func.InputStream) -> None:
    """Blob trigger — host entry point."""
    logging.info("blob %s length=%s", blob.name, blob.length)


@app.service_bus_queue_trigger(
    arg_name="message",
    queue_name="orders",
    connection="ServiceBusConnection",
)
def service_bus_worker(message: func.ServiceBusMessage) -> None:
    """Service Bus queue trigger — host entry point."""
    logging.info("service bus: %s", message.get_body().decode("utf-8"))


@app.event_hub_trigger(
    arg_name="events",
    event_hub_name="input",
    connection="EventHubConnection",
)
def event_hub_worker(events: func.EventHubEvent) -> None:
    """Event Hub trigger — host entry point."""
    logging.info("event hub batch received")


@app.cosmos_db_trigger(
    arg_name="documents",
    database_name="db",
    container_name="items",
    connection="CosmosDBConnection",
    lease_container_name="leases",
    create_lease_container_if_not_exists=True,
)
def cosmos_worker(documents: func.DocumentList) -> None:
    """Cosmos DB trigger — host entry point."""
    logging.info("cosmos docs=%s", len(documents))


@app.event_grid_trigger(arg_name="event")
def event_grid_worker(event: func.EventGridEvent) -> None:
    """Event Grid trigger — host entry point."""
    logging.info("event grid: %s", event.id)


# --- Durable Functions (DFApp) ---


@df_app.orchestration_trigger(context_name="context")
def hello_orchestrator(context: df.DurableOrchestrationContext):
    """Orchestration trigger — host entry point."""
    result = yield context.call_activity("say_hello", "Tokyo")
    return result


@df_app.activity_trigger(input_name="city")
def say_hello(city: str) -> str:
    """Activity trigger — host entry point (called by orchestrator by name)."""
    return format_message(city)


def completely_unused_helper() -> str:
    """Not an Azure entry point and never referenced — should be dead."""
    return "dead"
