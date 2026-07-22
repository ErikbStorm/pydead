"""Blueprint-registered HTTP function (still a host entry point)."""

import azure.functions as func

from shared.helpers import format_message

bp = func.Blueprint()


@bp.route(route="orders/{order_id}", methods=["GET"])
def get_order(req: func.HttpRequest) -> func.HttpResponse:
    """Blueprint HTTP trigger — registered via app.register_functions(bp)."""
    order_id = req.route_params.get("order_id", "unknown")
    return func.HttpResponse(format_message(f"order-{order_id}"))


def blueprint_dead_helper() -> None:
    """Never called — should be reported dead."""
    pass
