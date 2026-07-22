"""Shared helpers used by the core API."""

USED_CONSTANT = "hello"
UNUSED_CONSTANT = "goodbye"


def format_name(name: str) -> str:
    """Used from api.Greeter — must stay live."""
    _ = _internal_helper()
    return f"{USED_CONSTANT}, {name}"


def _internal_helper() -> str:
    """Private helper only used inside this module — must stay live."""
    return USED_CONSTANT.upper()


def dead_helper() -> str:
    """Never called from anywhere — should be reported dead."""
    return "unused"
