"""Shared helpers used by Azure Function entry points."""

USED_PREFIX = "hello"


def format_message(name: str) -> str:
    """Used from HTTP / durable activity — must stay live."""
    return f"{USED_PREFIX}, {name}!"


def used_by_http(source: str) -> str:
    """Used from timer_cleanup — must stay live."""
    return f"from-{source}"


def unused_orphan() -> str:
    """Never called (only imported) — should be dead."""
    return "orphan"


def also_unused() -> int:
    """Completely unused — should be dead."""
    return 0
