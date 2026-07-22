"""Public-ish API surface for the greeter app."""

from libs.core.util import format_name, USED_CONSTANT


class Greeter:
    """Live class — constructed from apps.greeter.main."""

    def __init__(self, name: str) -> None:
        self.name = name

    def greet(self) -> str:
        """Live method — called from main."""
        return format_name(self.name)

    def unused_method(self) -> str:
        """Never called — should be reported dead."""
        return "nope"


class DeadService:
    """Never imported or referenced — entire class is dead."""

    def execute_dead_work(self) -> None:
        print("should not run")


def orphan_public() -> str:
    """Public but unused across the monorepo."""
    return USED_CONSTANT


def _orphan_private() -> str:
    """Private and unused."""
    return "secret"


def reexported_via_all() -> str:
    """Not called, but listed in package __all__ — must stay live."""
    return "exported"
